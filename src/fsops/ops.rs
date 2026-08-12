//! Операции над файлами, которые выполняются мгновенно и не требуют демона:
//! корзина, удаление, переименование, создание, открытие во внешней программе.

use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::places;

/// Открыть файл или папку системным обработчиком.
pub fn open_external(path: &Path) -> io::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let mut cmd = Command::new("xdg-open");
    cmd.arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Открытая программа не должна умереть вместе с менеджером.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = cmd.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

pub fn rename(path: &Path, new_name: &str) -> io::Result<PathBuf> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "пустое имя"));
    }
    if new_name.contains('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "имя не может содержать «/»",
        ));
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    let target = parent.join(new_name);

    if target == path {
        return Ok(target);
    }
    if target.symlink_metadata().is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("«{new_name}» уже существует"),
        ));
    }

    std::fs::rename(path, &target)?;
    Ok(target)
}

pub fn create_dir(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "недопустимое имя"));
    }
    let path = parent.join(name);
    std::fs::create_dir(&path)?;
    Ok(path)
}

pub fn create_file(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "недопустимое имя"));
    }
    let path = parent.join(name);
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    Ok(path)
}

/// Есть ли право записи в каталог (нужно, чтобы включать «Вставить»).
pub fn is_writable(path: &Path) -> bool {
    let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    unsafe { libc::access(c_path.as_ptr(), libc::W_OK) == 0 }
}

/// Безвозвратное удаление.
pub fn delete(path: &Path) -> io::Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// Переместить в корзину по спецификации freedesktop.org.
///
/// Сначала пробуем домашнюю корзину; если объект на другой файловой системе —
/// `.Trash-<uid>` в корне её точки монтирования.
pub fn move_to_trash(path: &Path) -> io::Result<()> {
    let absolute = std::fs::canonicalize(path.parent().unwrap_or(Path::new("/")))
        .map(|p| p.join(path.file_name().unwrap_or_default()))
        .unwrap_or_else(|_| path.to_path_buf());

    let home_trash = places::data_dir().join("Trash");
    match trash_into(&home_trash, path, &absolute) {
        Ok(()) => return Ok(()),
        Err(err) if err.raw_os_error() != Some(libc::EXDEV) => return Err(err),
        Err(_) => {}
    }

    // Другая файловая система — корзина в корне её монтирования.
    let uid = unsafe { libc::getuid() };
    let root = mount_root(&absolute)?;
    let dev_trash = root.join(format!(".Trash-{uid}"));

    trash_into(&dev_trash, path, &absolute)
}

fn trash_into(trash: &Path, source: &Path, absolute: &Path) -> io::Result<()> {
    let files = trash.join("files");
    let info = trash.join("info");
    std::fs::create_dir_all(&files)?;
    std::fs::create_dir_all(&info)?;

    let name = source
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "нет имени файла"))?
        .to_string_lossy()
        .to_string();

    // Имя должно быть свободно и в files/, и в info/.
    let mut target_name = name.clone();
    let mut n = 1;
    while files.join(&target_name).symlink_metadata().is_ok()
        || info.join(format!("{target_name}.trashinfo")).exists()
    {
        n += 1;
        target_name = match name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() => format!("{stem}.{n}.{ext}"),
            _ => format!("{name}.{n}"),
        };
    }

    let info_path = info.join(format!("{target_name}.trashinfo"));
    let record = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        uri_encode(absolute),
        iso_now()
    );

    // Сначала info-файл: если переезд не удастся, запись просто уберём.
    std::fs::write(&info_path, record)?;

    if let Err(err) = std::fs::rename(source, files.join(&target_name)) {
        let _ = std::fs::remove_file(&info_path);
        return Err(err);
    }

    Ok(())
}

/// Корень файловой системы, на которой лежит путь: идём вверх, пока не
/// сменится номер устройства.
fn mount_root(path: &Path) -> io::Result<PathBuf> {
    let dev = std::fs::metadata(path)?.dev();
    let mut current = path.to_path_buf();

    while let Some(parent) = current.parent() {
        let Ok(meta) = std::fs::metadata(parent) else {
            break;
        };
        if meta.dev() != dev {
            break;
        }
        current = parent.to_path_buf();
    }

    Ok(current)
}

fn uri_encode(path: &Path) -> String {
    let mut out = String::new();
    for &byte in path.as_os_str().as_bytes() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as libc::time_t;

    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&secs, &mut tm).is_null() } {
        return "1970-01-01T00:00:00".into();
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}
