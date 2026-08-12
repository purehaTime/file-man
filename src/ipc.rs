//! Протокол общения GUI с фоновым демоном копирования.
//!
//! Транспорт — unix-сокет в `$XDG_RUNTIME_DIR/file-man/`, поверх него
//! построчный JSON (одно сообщение — одна строка). Демон живёт отдельным
//! процессом, поэтому закрытие окна не прерывает копирование.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Аргумент, по которому бинарник запускается в режиме демона.
pub const DAEMON_ARG: &str = "--copy-daemon";

/// Демон закрывается, если столько секунд нет ни клиентов, ни активных задач.
pub const IDLE_TIMEOUT_SECS: u64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    Copy,
    Move,
}

impl Op {
    pub fn verb(self) -> &'static str {
        match self {
            Op::Copy => "Копирование",
            Op::Move => "Перемещение",
        }
    }
}

/// Что делать, если файл с таким именем уже есть в приёмнике.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Conflict {
    /// Дописать к имени ` (2)`, ` (3)`, …
    Rename,
    Overwrite,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    /// Обход дерева источников: считаем файлы и байты.
    Scanning,
    Running,
    Paused,
    Done,
    Cancelled,
    Failed(String),
}

impl JobState {
    pub fn is_active(&self) -> bool {
        matches!(self, JobState::Scanning | JobState::Running | JobState::Paused)
    }

    pub fn label(&self) -> &'static str {
        match self {
            JobState::Scanning => "подсчёт",
            JobState::Running => "выполняется",
            JobState::Paused => "пауза",
            JobState::Done => "готово",
            JobState::Cancelled => "отменено",
            JobState::Failed(_) => "ошибка",
        }
    }
}

/// Снимок состояния одной фоновой задачи.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub op: Op,
    pub sources: Vec<PathBuf>,
    pub dest: PathBuf,
    pub state: JobState,
    pub total_bytes: u64,
    pub done_bytes: u64,
    pub total_items: u64,
    pub done_items: u64,
    /// Файл, который обрабатывается прямо сейчас.
    pub current: Option<PathBuf>,
    /// Ошибки по отдельным файлам; задача при этом продолжается.
    pub errors: Vec<String>,
    /// Байт в секунду, скользящее среднее.
    pub speed: u64,
    /// Осталось секунд, если оценка возможна.
    pub eta: Option<u64>,
}

impl Job {
    pub fn fraction(&self) -> f32 {
        if self.total_bytes == 0 {
            if self.total_items == 0 {
                return 0.0;
            }
            return self.done_items as f32 / self.total_items as f32;
        }
        (self.done_bytes as f32 / self.total_bytes as f32).clamp(0.0, 1.0)
    }

    /// Короткое имя для статусной строки: «Копирование → Загрузки».
    pub fn title(&self) -> String {
        let dest = self
            .dest
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.dest.display().to_string());

        match self.sources.len() {
            1 => {
                let src = self.sources[0]
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                format!("{} «{}» → {}", self.op.verb(), src, dest)
            }
            n => format!("{} {} об. → {}", self.op.verb(), n, dest),
        }
    }
}

/// Сообщение GUI → демон.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Подписаться на поток обновлений. В ответ сразу приходит `Snapshot`.
    Subscribe,
    /// Разовый запрос состояния без подписки.
    Snapshot,
    Submit {
        op: Op,
        sources: Vec<PathBuf>,
        dest: PathBuf,
        conflict: Conflict,
    },
    Cancel(u64),
    Pause(u64),
    Resume(u64),
    /// Убрать завершённую задачу из списка.
    Dismiss(u64),
    /// Убрать все завершённые задачи.
    DismissFinished,
}

/// Сообщение демон → GUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Snapshot { jobs: Vec<Job> },
    Updated { job: Job },
    Removed { id: u64 },
    Accepted { id: u64 },
    Error { message: String },
}

/// Каталог для сокета и lock-файла.
pub fn runtime_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("file-man");
    }
    // Запасной вариант для окружений без XDG_RUNTIME_DIR.
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/file-man-{uid}"))
}

pub fn socket_path() -> PathBuf {
    runtime_dir().join("copyd.sock")
}

pub fn lock_path() -> PathBuf {
    runtime_dir().join("copyd.lock")
}

/// Подключиться к уже работающему демону.
pub fn connect() -> io::Result<UnixStream> {
    UnixStream::connect(socket_path())
}

/// Подключиться, запустив демон, если тот ещё не поднят.
pub fn connect_or_spawn() -> io::Result<UnixStream> {
    if let Ok(stream) = connect() {
        return Ok(stream);
    }

    spawn_daemon()?;

    // Демону нужно время на bind; пробуем ~2 секунды.
    let mut last = io::Error::new(io::ErrorKind::NotConnected, "демон не отвечает");
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(50));
        match connect() {
            Ok(stream) => return Ok(stream),
            Err(err) => last = err,
        }
    }
    Err(last)
}

/// Запустить демон отдельным сеансом, чтобы он пережил закрытие GUI.
pub fn spawn_daemon() -> io::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg(DAEMON_ARG)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // setsid отвязывает демон от группы процессов терминала/GUI: SIGHUP и
    // SIGINT родителя до него уже не долетят.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = cmd.spawn()?;
    // Демон сам себя переоткрывает и работает независимо; ждём только код
    // выхода первого уровня, чтобы не плодить зомби.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Отправить запрос и, при необходимости, дождаться одного ответа.
pub fn request(req: &Request, wait_reply: bool) -> io::Result<Option<Response>> {
    let mut stream = connect_or_spawn()?;
    write_line(&mut stream, req)?;

    if !wait_reply {
        return Ok(None);
    }

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    Ok(serde_json::from_str(&line).ok())
}

pub fn write_line<W: Write, T: Serialize>(writer: &mut W, value: &T) -> io::Result<()> {
    let mut buf = serde_json::to_vec(value).map_err(io::Error::other)?;
    buf.push(b'\n');
    writer.write_all(&buf)?;
    writer.flush()
}

/// Свободное/общее место на файловой системе, где лежит `path`.
pub fn disk_usage(path: &Path) -> Option<(u64, u64)> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }
    let block = stat.f_frsize as u64;
    let total = stat.f_blocks as u64 * block;
    let free = stat.f_bavail as u64 * block;
    Some((free, total))
}
