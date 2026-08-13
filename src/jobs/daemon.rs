//! Демон копирования.
//!
//! Живёт отдельным процессом: GUI подключается к сокету, отдаёт задачи и
//! подписывается на обновления. Закрытие окна не трогает демон, а новый
//! экземпляр GUI при подключении сразу получает снимок текущих задач.

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::ipc::{self, Conflict, Job, JobState, Naming, Op, Request, Response};

/// Флаги управления одной задачей, которые читает движок между чанками.
pub struct Control {
    pub cancel: AtomicBool,
    pub paused: AtomicBool,
    /// Будит движок, когда паузу сняли или задачу отменили.
    wake: Condvar,
    wake_lock: Mutex<()>,
}

impl Control {
    fn new() -> Self {
        Self {
            cancel: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            wake: Condvar::new(),
            wake_lock: Mutex::new(()),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Блокируется, пока стоит пауза. `false` — задачу отменили.
    pub fn wait_while_paused(&self) -> bool {
        if self.is_cancelled() {
            return false;
        }
        if !self.paused.load(Ordering::Relaxed) {
            return true;
        }
        let mut guard = self.wake_lock.lock().unwrap();
        while self.paused.load(Ordering::Relaxed) && !self.is_cancelled() {
            let (g, _) = self
                .wake
                .wait_timeout(guard, Duration::from_millis(250))
                .unwrap();
            guard = g;
        }
        !self.is_cancelled()
    }

    fn notify(&self) {
        let _guard = self.wake_lock.lock().unwrap();
        self.wake.notify_all();
    }
}

struct Subscriber {
    id: u64,
    tx: Sender<Response>,
}

struct Inner {
    jobs: BTreeMap<u64, Job>,
    controls: BTreeMap<u64, Arc<Control>>,
    subscribers: Vec<Subscriber>,
    clients: usize,
    idle_since: Instant,
}

/// Общее состояние демона.
pub struct Registry {
    inner: Mutex<Inner>,
    next_job: AtomicU64,
    next_sub: AtomicU64,
}

impl Registry {
    fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                jobs: BTreeMap::new(),
                controls: BTreeMap::new(),
                subscribers: Vec::new(),
                clients: 0,
                idle_since: Instant::now(),
            }),
            next_job: AtomicU64::new(1),
            next_sub: AtomicU64::new(1),
        }
    }

    fn snapshot(&self) -> Vec<Job> {
        self.inner.lock().unwrap().jobs.values().cloned().collect()
    }

    fn subscribe(&self, tx: Sender<Response>) -> u64 {
        let id = self.next_sub.fetch_add(1, Ordering::Relaxed);
        let mut inner = self.inner.lock().unwrap();
        inner.subscribers.push(Subscriber { id, tx });
        id
    }

    fn unsubscribe(&self, id: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.subscribers.retain(|s| s.id != id);
    }

    fn client_connected(&self) {
        self.inner.lock().unwrap().clients += 1;
    }

    fn client_disconnected(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.clients = inner.clients.saturating_sub(1);
        inner.idle_since = Instant::now();
    }

    fn broadcast(inner: &mut Inner, msg: &Response) {
        inner.subscribers.retain(|s| s.tx.send(msg.clone()).is_ok());
    }

    /// Изменить задачу и разослать новый снимок подписчикам.
    pub fn update_job<F: FnOnce(&mut Job)>(&self, id: u64, f: F) {
        let mut inner = self.inner.lock().unwrap();
        let Some(job) = inner.jobs.get_mut(&id) else {
            return;
        };
        f(job);
        let finished = !job.state.is_active();
        let msg = Response::Updated { job: job.clone() };
        if finished {
            inner.controls.remove(&id);
            inner.idle_since = Instant::now();
        }
        Self::broadcast(&mut inner, &msg);
    }

    fn submit(
        self: &Arc<Self>,
        op: Op,
        sources: Vec<PathBuf>,
        dest: PathBuf,
        conflict: Conflict,
        naming: Naming,
    ) -> u64 {
        let id = self.next_job.fetch_add(1, Ordering::Relaxed);
        let ctl = Arc::new(Control::new());

        let job = Job {
            id,
            op,
            sources: sources.clone(),
            dest: dest.clone(),
            state: JobState::Scanning,
            total_bytes: 0,
            done_bytes: 0,
            total_items: 0,
            done_items: 0,
            current: None,
            errors: Vec::new(),
            speed: 0,
            eta: None,
        };

        {
            let mut inner = self.inner.lock().unwrap();
            inner.jobs.insert(id, job.clone());
            inner.controls.insert(id, ctl.clone());
            let msg = Response::Updated { job };
            Self::broadcast(&mut inner, &msg);
        }

        let registry = Arc::clone(self);
        std::thread::Builder::new()
            .name(format!("copy-job-{id}"))
            // Обход дерева рекурсивный, запас стека на очень глубокие каталоги.
            .stack_size(8 << 20)
            .spawn(move || {
                super::engine::run(registry, id, ctl, op, sources, dest, conflict, naming);
            })
            .expect("не удалось создать поток задачи");

        id
    }

    fn cancel(&self, id: u64) {
        let ctl = self.inner.lock().unwrap().controls.get(&id).cloned();
        if let Some(ctl) = ctl {
            ctl.cancel.store(true, Ordering::Relaxed);
            ctl.paused.store(false, Ordering::Relaxed);
            ctl.notify();
        }
    }

    fn set_paused(&self, id: u64, paused: bool) {
        let ctl = self.inner.lock().unwrap().controls.get(&id).cloned();
        if let Some(ctl) = ctl {
            ctl.paused.store(paused, Ordering::Relaxed);
            ctl.notify();
        }
        self.update_job(id, |job| {
            if paused {
                if matches!(job.state, JobState::Running | JobState::Scanning) {
                    job.state = JobState::Paused;
                }
            } else if job.state == JobState::Paused {
                job.state = JobState::Running;
            }
        });
    }

    fn dismiss(&self, id: u64) {
        let mut inner = self.inner.lock().unwrap();
        let active = inner.jobs.get(&id).map(|j| j.state.is_active()).unwrap_or(false);
        if active {
            return;
        }
        if inner.jobs.remove(&id).is_some() {
            inner.idle_since = Instant::now();
            Self::broadcast(&mut inner, &Response::Removed { id });
        }
    }

    fn dismiss_finished(&self) {
        let finished: Vec<u64> = {
            let inner = self.inner.lock().unwrap();
            inner
                .jobs
                .values()
                .filter(|j| !j.state.is_active())
                .map(|j| j.id)
                .collect()
        };
        for id in finished {
            self.dismiss(id);
        }
    }

    /// Пора ли выключаться: нет клиентов, нет активных задач, тишина дольше таймаута.
    fn should_exit(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        if inner.clients > 0 {
            return false;
        }
        if inner.jobs.values().any(|j| j.state.is_active()) {
            return false;
        }
        inner.idle_since.elapsed() > Duration::from_secs(ipc::IDLE_TIMEOUT_SECS)
    }
}

/// Точка входа режима `--copy-daemon`.
pub fn run() -> io::Result<()> {
    let dir = ipc::runtime_dir();
    std::fs::create_dir_all(&dir)?;

    // Единственность демона держится на flock: файл не удаляем, блокировка
    // снимается ядром при завершении процесса.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(ipc::lock_path())?;

    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        // Демон уже поднят кем-то другим — выходим молча.
        return Ok(());
    }

    let socket = ipc::socket_path();
    // Сокет мог остаться от процесса, убитого по SIGKILL.
    if socket.exists() && UnixStream::connect(&socket).is_err() {
        let _ = std::fs::remove_file(&socket);
    }
    let listener = UnixListener::bind(&socket)?;

    let registry = Arc::new(Registry::new());

    {
        let registry = Arc::clone(&registry);
        let socket = socket.clone();
        std::thread::Builder::new()
            .name("idle-watch".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(5));
                if registry.should_exit() {
                    let _ = std::fs::remove_file(&socket);
                    std::process::exit(0);
                }
            })
            .expect("не удалось создать поток контроля простоя");
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry = Arc::clone(&registry);
                std::thread::Builder::new()
                    .name("client".into())
                    .spawn(move || handle_client(stream, registry))
                    .ok();
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    let _ = std::fs::remove_file(&socket);
    Ok(())
}

fn handle_client(stream: UnixStream, registry: Arc<Registry>) {
    registry.client_connected();

    let writer_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => {
            registry.client_disconnected();
            return;
        }
    };

    let (tx, rx) = mpsc::channel::<Response>();
    let writer = std::thread::spawn(move || {
        let mut out = writer_stream;
        while let Ok(msg) = rx.recv() {
            if ipc::write_line(&mut out, &msg).is_err() {
                break;
            }
        }
    });

    let mut subscription: Option<u64> = None;
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }

        let req: Request = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(err) => {
                let _ = tx.send(Response::Error {
                    message: format!("некорректный запрос: {err}"),
                });
                continue;
            }
        };

        let keep_going = match req {
            Request::Subscribe => {
                if subscription.is_none() {
                    subscription = Some(registry.subscribe(tx.clone()));
                }
                tx.send(Response::Snapshot {
                    jobs: registry.snapshot(),
                })
                .is_ok()
            }
            Request::Snapshot => tx
                .send(Response::Snapshot {
                    jobs: registry.snapshot(),
                })
                .is_ok(),
            Request::Submit {
                op,
                sources,
                dest,
                conflict,
                naming,
            } => {
                if sources.is_empty() {
                    tx.send(Response::Error {
                        message: "пустой список источников".into(),
                    })
                    .is_ok()
                } else {
                    let id = registry.submit(op, sources, dest, conflict, naming);
                    tx.send(Response::Accepted { id }).is_ok()
                }
            }
            Request::Cancel(id) => {
                registry.cancel(id);
                true
            }
            Request::Pause(id) => {
                registry.set_paused(id, true);
                true
            }
            Request::Resume(id) => {
                registry.set_paused(id, false);
                true
            }
            Request::Dismiss(id) => {
                registry.dismiss(id);
                true
            }
            Request::DismissFinished => {
                registry.dismiss_finished();
                true
            }
        };

        if !keep_going {
            break;
        }
    }

    if let Some(id) = subscription {
        registry.unsubscribe(id);
    }
    drop(tx);
    let _ = writer.join();
    registry.client_disconnected();
}
