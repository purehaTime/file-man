//! Движок копирования и перемещения.
//!
//! Работает в отдельном потоке демона: сначала обходит источники и считает
//! объём, затем копирует, регулярно отдавая прогресс в реестр. Между чанками
//! проверяются флаги отмены и паузы.

use std::ffi::CString;
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::daemon::{Control, Registry};
use crate::ipc::{Conflict, JobState, Op};

/// Размер буфера при обычном чтении/записи.
const BUF_SIZE: usize = 1 << 20; // 1 МиБ
/// Порция для `copy_file_range`: компромисс между скоростью и детализацией прогресса.
const RANGE_CHUNK: usize = 8 << 20; // 8 МиБ
/// Как часто отдавать прогресс в реестр.
const PUSH_INTERVAL: Duration = Duration::from_millis(120);

/// Ядро не умеет `copy_file_range` на этих ФС — переключаемся на read/write
/// один раз за жизнь процесса.
static COPY_RANGE_SUPPORTED: AtomicBool = AtomicBool::new(true);

enum Error {
    Cancelled,
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Default, Clone, Copy)]
struct Stat {
    items: u64,
    bytes: u64,
}

/// Точка входа задачи.
pub fn run(
    registry: Arc<Registry>,
    id: u64,
    ctl: Arc<Control>,
    op: Op,
    sources: Vec<PathBuf>,
    dest: PathBuf,
    conflict: Conflict,
) {
    // Фаза 1 — обход дерева.
    let mut stats = Vec::with_capacity(sources.len());
    let mut total = Stat::default();
    for src in &sources {
        if ctl.is_cancelled() {
            break;
        }
        let stat = scan(src, &ctl);
        total.items += stat.items;
        total.bytes += stat.bytes;
        stats.push(stat);

        registry.update_job(id, |job| {
            job.total_items = total.items;
            job.total_bytes = total.bytes;
        });
    }

    if ctl.is_cancelled() {
        registry.update_job(id, |job| job.state = JobState::Cancelled);
        return;
    }

    if let Err(err) = fs::create_dir_all(&dest) {
        registry.update_job(id, |job| {
            job.state = JobState::Failed(format!("{}: {err}", dest.display()));
        });
        return;
    }

    registry.update_job(id, |job| job.state = JobState::Running);

    // Фаза 2 — собственно перенос.
    let mut progress = Progress::new(Arc::clone(&registry), id, total.bytes);
    let mut cancelled = false;

    for (src, stat) in sources.iter().zip(stats.into_iter()) {
        if ctl.is_cancelled() {
            cancelled = true;
            break;
        }
        let errors_before = progress.errors.len();

        match transfer_root(src, &dest, op, conflict, &stat, &mut progress, &ctl) {
            Ok(()) => {}
            Err(Error::Cancelled) => {
                cancelled = true;
                break;
            }
            Err(Error::Io(err)) => {
                progress.error(format!("{}: {err}", src.display()));
            }
        }

        // Источник удаляем только если весь его подкаталог перенесён без ошибок.
        if op == Op::Move && progress.errors.len() == errors_before {
            if let Err(err) = remove_path(src) {
                progress.error(format!("не удалось удалить {}: {err}", src.display()));
            }
        }
    }

    let errors = std::mem::take(&mut progress.errors);
    let all_failed = !errors.is_empty() && progress.done_items == 0;

    registry.update_job(id, |job| {
        job.current = None;
        job.speed = 0;
        job.eta = None;
        job.done_bytes = progress.done_bytes;
        job.done_items = progress.done_items;
        job.errors = errors;
        job.state = if cancelled {
            JobState::Cancelled
        } else if all_failed {
            JobState::Failed(
                job.errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "перенос не удался".into()),
            )
        } else {
            JobState::Done
        };
    });
}

// ---------------------------------------------------------------- обход

fn scan(root: &Path, ctl: &Control) -> Stat {
    let mut stat = Stat::default();
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        if ctl.is_cancelled() {
            break;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };

        stat.items += 1;

        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    stack.push(entry.path());
                }
            }
        } else {
            stat.bytes += meta.len();
        }
    }

    stat
}

// ---------------------------------------------------------------- перенос

fn transfer_root(
    src: &Path,
    dest_dir: &Path,
    op: Op,
    conflict: Conflict,
    stat: &Stat,
    progress: &mut Progress,
    ctl: &Control,
) -> Result<()> {
    let Some(name) = src.file_name() else {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "у источника нет имени",
        )));
    };

    let naive_target = dest_dir.join(name);

    if is_inside(src, &naive_target) {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "нельзя поместить каталог внутрь самого себя",
        )));
    }

    let Some(target) = resolve_conflict(&naive_target, conflict) else {
        progress.jump(*stat);
        return Ok(());
    };

    if op == Op::Move {
        match fs::rename(src, &target) {
            Ok(()) => {
                // Переименование в пределах одной ФС — мгновенный перенос.
                progress.set_current(src);
                progress.jump(*stat);
                return Ok(());
            }
            Err(err) => {
                let code = err.raw_os_error().unwrap_or(0);
                let recoverable = matches!(
                    code,
                    libc::EXDEV | libc::ENOTEMPTY | libc::EEXIST | libc::EISDIR | libc::ENOTDIR
                );
                if !recoverable {
                    return Err(Error::Io(err));
                }
                // Иначе — обычное копирование с последующим удалением.
            }
        }
    }

    transfer(src, &target, conflict, progress, ctl)
}

fn transfer(
    src: &Path,
    target: &Path,
    conflict: Conflict,
    progress: &mut Progress,
    ctl: &Control,
) -> Result<()> {
    if !ctl.wait_while_paused() {
        return Err(Error::Cancelled);
    }

    let meta = fs::symlink_metadata(src)?;

    if meta.file_type().is_symlink() {
        let link = fs::read_link(src)?;
        if exists(target) {
            remove_path(target)?;
        }
        std::os::unix::fs::symlink(link, target)?;
        progress.item();
        return Ok(());
    }

    if meta.is_dir() {
        progress.set_current(src);

        if exists(target) && !target.is_dir() {
            remove_path(target)?;
        }
        fs::create_dir_all(target)?;

        let entries = fs::read_dir(src)?;
        for entry in entries {
            if ctl.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    progress.error(format!("{}: {err}", src.display()));
                    continue;
                }
            };

            let child = entry.path();
            let naive = target.join(entry.file_name());
            let Some(child_target) = resolve_conflict(&naive, conflict) else {
                progress.jump(scan(&child, ctl));
                continue;
            };

            match transfer(&child, &child_target, conflict, progress, ctl) {
                Ok(()) => {}
                Err(Error::Cancelled) => return Err(Error::Cancelled),
                Err(Error::Io(err)) => {
                    progress.error(format!("{}: {err}", child.display()));
                }
            }
        }

        let _ = fs::set_permissions(target, meta.permissions());
        let _ = set_times(target, &meta);
        progress.item();
        return Ok(());
    }

    progress.set_current(src);
    copy_file(src, target, &meta, progress, ctl)?;
    progress.item();
    Ok(())
}

fn copy_file(
    src: &Path,
    target: &Path,
    meta: &Metadata,
    progress: &mut Progress,
    ctl: &Control,
) -> Result<()> {
    let result = copy_file_inner(src, target, meta, progress, ctl);

    if matches!(result, Err(Error::Cancelled)) {
        // Недокопированный хвост оставлять нельзя — он выглядит как готовый файл.
        let _ = fs::remove_file(target);
    }

    result
}

fn copy_file_inner(
    src: &Path,
    target: &Path,
    meta: &Metadata,
    progress: &mut Progress,
    ctl: &Control,
) -> Result<()> {
    let mut reader = File::open(src)?;
    let mut writer = File::create(target)?;

    let mut copied_by_range = false;
    if COPY_RANGE_SUPPORTED.load(Ordering::Relaxed) && meta.len() > 0 {
        copied_by_range = copy_by_range(&reader, &writer, meta.len(), progress, ctl)?;
    }

    if !copied_by_range {
        let mut buf = vec![0u8; BUF_SIZE];
        loop {
            if !ctl.wait_while_paused() {
                return Err(Error::Cancelled);
            }
            let read = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(Error::Io(err)),
            };
            writer.write_all(&buf[..read])?;
            progress.bytes(read as u64);
        }
    }

    writer.flush()?;
    let _ = writer.set_permissions(meta.permissions());
    drop(writer);
    let _ = set_times(target, meta);
    Ok(())
}

/// Копирование средствами ядра. `Ok(false)` — способ не поддерживается,
/// ни одного байта не перенесено, нужен откат на read/write.
fn copy_by_range(
    reader: &File,
    writer: &File,
    len: u64,
    progress: &mut Progress,
    ctl: &Control,
) -> Result<bool> {
    let mut remaining = len;
    let mut first = true;

    while remaining > 0 {
        if !ctl.wait_while_paused() {
            return Err(Error::Cancelled);
        }

        let chunk = remaining.min(RANGE_CHUNK as u64) as usize;
        let copied = unsafe {
            libc::copy_file_range(
                reader.as_raw_fd(),
                std::ptr::null_mut(),
                writer.as_raw_fd(),
                std::ptr::null_mut(),
                chunk,
                0,
            )
        };

        if copied < 0 {
            let err = io::Error::last_os_error();
            let code = err.raw_os_error().unwrap_or(0);
            if code == libc::EINTR {
                continue;
            }
            if first
                && matches!(
                    code,
                    libc::ENOSYS | libc::EXDEV | libc::EINVAL | libc::EOPNOTSUPP | libc::EPERM
                )
            {
                COPY_RANGE_SUPPORTED.store(false, Ordering::Relaxed);
                return Ok(false);
            }
            return Err(Error::Io(err));
        }

        if copied == 0 {
            break; // файл оказался короче, чем сказали метаданные
        }

        first = false;
        remaining -= copied as u64;
        progress.bytes(copied as u64);
    }

    Ok(true)
}

// ---------------------------------------------------------------- утилиты

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn remove_path(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Куда на самом деле писать с учётом политики конфликтов.
/// `None` — файл нужно пропустить.
fn resolve_conflict(target: &Path, conflict: Conflict) -> Option<PathBuf> {
    if !exists(target) {
        return Some(target.to_path_buf());
    }
    match conflict {
        Conflict::Overwrite => Some(target.to_path_buf()),
        Conflict::Skip => None,
        Conflict::Rename => Some(unique_path(target)),
    }
}

/// `foo.txt` → `foo (2).txt`, `foo (3).txt`, …
pub fn unique_path(target: &Path) -> PathBuf {
    if !exists(target) {
        return target.to_path_buf();
    }

    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target.file_name().unwrap_or_default().to_string_lossy();

    // Расширение берём «составное» только для распространённых случаев вроде
    // .tar.gz, иначе достаточно последней точки.
    let (stem, ext) = match name.rfind('.') {
        Some(pos) if pos > 0 => {
            let (stem, ext) = name.split_at(pos);
            if let Some(tar) = stem.strip_suffix(".tar") {
                (tar.to_string(), format!(".tar{ext}"))
            } else {
                (stem.to_string(), ext.to_string())
            }
        }
        _ => (name.to_string(), String::new()),
    };

    for n in 2..10_000u32 {
        let candidate = parent.join(format!("{stem} ({n}){ext}"));
        if !exists(&candidate) {
            return candidate;
        }
    }

    parent.join(format!("{stem} ({}){ext}", std::process::id()))
}

/// Лежит ли `target` внутри `src` (защита от копирования папки в себя).
fn is_inside(src: &Path, target: &Path) -> bool {
    let src = fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    let target = target
        .parent()
        .and_then(|p| fs::canonicalize(p).ok())
        .map(|p| p.join(target.file_name().unwrap_or_default()))
        .unwrap_or_else(|| target.to_path_buf());
    target.starts_with(&src)
}

fn set_times(path: &Path, meta: &Metadata) -> io::Result<()> {
    let times = [
        libc::timespec {
            tv_sec: meta.atime() as libc::time_t,
            tv_nsec: meta.atime_nsec() as libc::c_long,
        },
        libc::timespec {
            tv_sec: meta.mtime() as libc::time_t,
            tv_nsec: meta.mtime_nsec() as libc::c_long,
        },
    ];

    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)?;
    let rc = unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            c_path.as_ptr(),
            times.as_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };

    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

// ---------------------------------------------------------------- прогресс

struct Progress {
    registry: Arc<Registry>,
    id: u64,
    total_bytes: u64,
    done_bytes: u64,
    done_items: u64,
    errors: Vec<String>,
    current: Option<PathBuf>,
    last_push: Instant,
    window_start: Instant,
    window_bytes: u64,
    speed: u64,
}

impl Progress {
    fn new(registry: Arc<Registry>, id: u64, total_bytes: u64) -> Self {
        let now = Instant::now();
        Self {
            registry,
            id,
            total_bytes,
            done_bytes: 0,
            done_items: 0,
            errors: Vec::new(),
            current: None,
            last_push: now,
            window_start: now,
            window_bytes: 0,
            speed: 0,
        }
    }

    fn bytes(&mut self, n: u64) {
        self.done_bytes += n;
        self.window_bytes += n;
        self.push(false);
    }

    fn item(&mut self) {
        self.done_items += 1;
        self.push(false);
    }

    /// Учесть целый подкаталог разом: быстрое переименование или пропуск.
    fn jump(&mut self, stat: Stat) {
        self.done_items += stat.items;
        self.done_bytes += stat.bytes;
        self.push(true);
    }

    fn set_current(&mut self, path: &Path) {
        self.current = Some(path.to_path_buf());
        self.push(false);
    }

    fn error(&mut self, message: String) {
        // Список ошибок не должен расти бесконечно на большом дереве.
        if self.errors.len() < 200 {
            self.errors.push(message);
        }
        self.push(true);
    }

    fn push(&mut self, force: bool) {
        if !force && self.last_push.elapsed() < PUSH_INTERVAL {
            return;
        }
        self.last_push = Instant::now();

        let window = self.window_start.elapsed();
        if window >= Duration::from_millis(600) {
            self.speed = (self.window_bytes as f64 / window.as_secs_f64()) as u64;
            self.window_start = Instant::now();
            self.window_bytes = 0;
        }

        let eta = if self.speed > 0 && self.total_bytes > self.done_bytes {
            Some((self.total_bytes - self.done_bytes) / self.speed)
        } else {
            None
        };

        let done_bytes = self.done_bytes;
        let done_items = self.done_items;
        let current = self.current.clone();
        let speed = self.speed;
        let errors = self.errors.clone();

        self.registry.update_job(self.id, move |job| {
            job.done_bytes = done_bytes;
            job.done_items = done_items;
            job.current = current;
            job.speed = speed;
            job.eta = eta;
            job.errors = errors;
        });
    }
}
