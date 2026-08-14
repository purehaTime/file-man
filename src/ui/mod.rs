//! Состояние приложения, обработка сообщений и сборка интерфейса.

pub mod dialogs;
pub mod icons;
pub mod panel;
pub mod selection;
pub mod sidebar;
pub mod statusbar;
pub mod style;
pub mod toolbar;

use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Duration;

use iced::futures::StreamExt;
use iced::keyboard::{key::Named, Key, Modifiers};
use iced::widget::{container, mouse_area, opaque, pin, stack};
use iced::{Element, Fill, Point, Rectangle, Size, Subscription, Task, Theme};

use crate::config::{Config, ThemeChoice, ViewMode};
use crate::fsops::entry::{self, Entry, SortKey};
use crate::fsops::{ops, places, Place};
use crate::i18n::{Lang, S};
use crate::ipc::{self, Job, Op, Request, Response};

/// Высота строки в режимах «Подробно» и «Компактно» — нужна для прокрутки
/// к выделенному элементу.
pub const ROW_HEIGHT: f32 = 30.0;
/// Высота статусной строки вместе с разделителем.
const STATUS_HEIGHT: f32 = 31.0;
/// Высота строки фильтра, когда она показана.
const FILTER_HEIGHT: f32 = 37.0;
/// Высота шапки таблицы в режиме «Подробно».
const HEADER_HEIGHT: f32 = 31.0;
pub const LIST_ID: &str = "file-list";
pub const FILTER_ID: &str = "filter-input";
pub const PATH_ID: &str = "path-input";
pub const DIALOG_ID: &str = "dialog-input";

/// Модальное окно поверх интерфейса.
#[derive(Debug, Clone)]
pub enum Modal {
    Rename { path: PathBuf, value: String },
    NewFolder { value: String },
    NewFile { value: String },
    ConfirmDelete { paths: Vec<PathBuf>, permanent: bool },
    Properties { entry: Entry, contents: Option<usize> },
    Jobs,
    Error { title: String, message: String },
}

/// Рамка выделения. Появляется, когда мышь потянули с пустого места;
/// координаты — в пространстве содержимого, то есть с учётом прокрутки.
#[derive(Debug, Clone)]
pub struct Marquee {
    /// Задаётся на первом движении мыши после нажатия.
    pub origin: Option<Point>,
    pub current: Point,
    /// Что было выделено до протяжки: с Ctrl рамка дополняет это множество,
    /// без него оно пустое и выделение заменяется.
    pub base: HashSet<PathBuf>,
}

impl Marquee {
    /// Прямоугольник рамки; `None`, пока это просто клик без протяжки.
    pub fn area(&self) -> Option<Rectangle> {
        let origin = self.origin?;

        let width = (origin.x - self.current.x).abs();
        let height = (origin.y - self.current.y).abs();
        if width < 3.0 && height < 3.0 {
            return None;
        }

        Some(Rectangle {
            x: origin.x.min(self.current.x),
            y: origin.y.min(self.current.y),
            width,
            height,
        })
    }
}

/// Контекстное меню: по элементу списка или по пустому месту.
#[derive(Debug, Clone)]
pub struct Context {
    pub position: Point,
    pub target: Option<PathBuf>,
}

/// События от подписки на демон.
#[derive(Debug, Clone)]
pub enum JobEvent {
    Connected,
    Disconnected,
    Message(Response),
}

#[derive(Debug, Clone)]
pub struct LoadResult {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
    pub error: Option<String>,
    pub usage: Option<(u64, u64)>,
    pub select: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // навигация
    NavigateTo(PathBuf),
    Activate(usize),
    Back,
    Forward,
    Up,
    Refresh,
    Loaded(LoadResult),

    // выделение
    RowPressed(usize),
    RowHovered(Option<usize>),
    RowRightPressed(usize),
    EmptyPressed,
    EmptyRightPressed,
    SelectAll,
    /// Нажатие на пустом месте списка — начало возможной рамки.
    PanelPressed,
    /// Движение мыши в списке, координаты относительно видимой области.
    PanelMoved(Point),
    PanelReleased,
    Scrolled(f32),

    // буфер обмена и операции
    CopySelection,
    CutSelection,
    Paste,
    OpenSelected,
    RequestRename,
    RequestNewFolder,
    RequestNewFile,
    RequestDelete { permanent: bool },
    RequestProperties,
    OperationDone(Result<Option<PathBuf>, String>),

    // диалоги и меню
    ModalInput(String),
    ModalSubmit,
    ModalClose,
    ContextClose,
    ShowJobs,

    // вид
    SetView(ViewMode),
    SetSort(SortKey),
    SetTheme(ThemeChoice),
    ToggleThemeMenu,
    ToggleHidden,
    ToggleFilter,
    FilterChanged(String),
    TogglePathEdit,
    PathEditChanged(String),
    PathEditSubmit,

    // левая панель
    PlaceHovered(Option<usize>),
    ToggleBookmark,

    // фоновые задачи
    Jobs(JobEvent),
    CancelJob(u64),
    PauseJob(u64),
    ResumeJob(u64),
    DismissJob(u64),
    DismissFinished,

    // системное
    ModifiersChanged(Modifiers),
    CursorMoved(Point),
    WindowResized(Size),
    KeyPressed(Key, Modifiers),
    RefreshDrives,
    Noop,
}

pub struct App {
    pub config: Config,
    pub theme: Theme,

    pub dir: PathBuf,
    pub entries: Vec<Entry>,
    /// Индексы `entries`, прошедшие фильтр по имени.
    pub filtered: Vec<usize>,
    pub read_error: Option<String>,
    pub usage: Option<(u64, u64)>,
    pub loading: bool,

    pub selection: selection::State,
    pub hover: Option<usize>,
    pub hover_place: Option<usize>,
    pub marquee: Option<Marquee>,
    /// Текущая вертикальная прокрутка списка.
    pub scroll: f32,

    pub history: Vec<PathBuf>,
    pub history_pos: usize,

    pub quick: Vec<Place>,
    pub drives: Vec<Place>,

    pub clipboard: Option<(Op, Vec<PathBuf>)>,
    pub jobs: Vec<Job>,
    pub daemon_online: bool,

    pub filter: String,
    pub filter_visible: bool,
    pub path_edit: Option<String>,

    pub modal: Option<Modal>,
    pub context: Option<Context>,
    /// Открыта ли всплывающая панель выбора темы.
    pub theme_menu: bool,

    pub modifiers: Modifiers,
    pub cursor: Point,
    pub window: Size,
}

/// Каталог, переданный в командной строке; перекрывает сохранённый.
static START_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub fn set_start_dir(path: PathBuf) {
    let _ = START_DIR.set(path);
}

pub fn boot() -> (App, Task<Message>) {
    let config = Config::load();
    let theme = config.theme.theme();

    let start = START_DIR
        .get()
        .cloned()
        .or_else(|| config.last_dir.clone())
        .filter(|p| p.is_dir())
        .unwrap_or_else(places::home);

    let window = Size::new(config.window_size.0, config.window_size.1);
    let show_hidden = config.show_hidden;

    let app = App {
        quick: places::quick_access(config.lang, &config.bookmarks),
        drives: places::drives(config.lang),
        theme,
        dir: start.clone(),
        entries: Vec::new(),
        filtered: Vec::new(),
        read_error: None,
        usage: None,
        loading: true,
        selection: selection::State::default(),
        hover: None,
        hover_place: None,
        marquee: None,
        scroll: 0.0,
        history: vec![start.clone()],
        history_pos: 0,
        clipboard: None,
        jobs: Vec::new(),
        daemon_online: false,
        filter: String::new(),
        filter_visible: false,
        path_edit: None,
        modal: None,
        context: None,
        theme_menu: false,
        modifiers: Modifiers::default(),
        cursor: Point::ORIGIN,
        window,
        config,
    };

    (app, load(start, show_hidden, None))
}

pub fn title(app: &App) -> String {
    let name = app
        .dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| app.dir.display().to_string());

    app.config.lang.window_title(&name)
}

pub fn theme(app: &App) -> Theme {
    app.theme.clone()
}

pub fn subscription(_app: &App) -> Subscription<Message> {
    Subscription::batch([
        jobs_subscription(),
        events_subscription(),
        ticker_subscription(),
    ])
}

/// Периодическое обновление списка дисков и свободного места.
/// Свой поток вместо `iced::time::every`, чтобы не тянуть tokio/smol.
fn ticker_subscription() -> Subscription<Message> {
    Subscription::run(|| {
        let (tx, rx) = iced::futures::channel::mpsc::channel::<()>(4);

        std::thread::Builder::new()
            .name("ticker".into())
            .spawn(move || {
                let mut tx = tx;
                loop {
                    std::thread::sleep(Duration::from_secs(6));
                    match tx.try_send(()) {
                        Ok(()) => {}
                        Err(err) if err.is_full() => {}
                        Err(_) => break, // приёмник закрыт
                    }
                }
            })
            .expect("не удалось создать поток таймера");

        rx.map(|_| Message::RefreshDrives)
    })
}

/// Поток обновлений от демона. Соединение живёт в отдельном потоке, чтобы не
/// блокировать исполнитель iced, и само переподключается.
fn jobs_subscription() -> Subscription<Message> {
    Subscription::run(|| {
        let (tx, rx) = iced::futures::channel::mpsc::channel::<JobEvent>(512);

        std::thread::Builder::new()
            .name("daemon-link".into())
            .spawn(move || {
                let mut tx = tx;
                loop {
                    if let Ok(mut stream) = ipc::connect_or_spawn() {
                        if ipc::write_line(&mut stream, &Request::Subscribe).is_ok() {
                            send_event(&mut tx, JobEvent::Connected);

                            let reader = BufReader::new(stream);
                            for line in reader.lines() {
                                let Ok(line) = line else { break };
                                match serde_json::from_str::<Response>(&line) {
                                    Ok(response) => {
                                        send_event(&mut tx, JobEvent::Message(response));
                                    }
                                    Err(_) => continue,
                                }
                            }
                        }
                    }

                    send_event(&mut tx, JobEvent::Disconnected);
                    std::thread::sleep(Duration::from_secs(2));
                }
            })
            .expect("не удалось создать поток связи с демоном");

        rx.map(Message::Jobs)
    })
}

/// Канал ограничен; если приёмник не успевает — ждём, а не теряем событие.
fn send_event(tx: &mut iced::futures::channel::mpsc::Sender<JobEvent>, event: JobEvent) {
    let mut pending = event;
    for _ in 0..500 {
        match tx.try_send(pending) {
            Ok(()) => return,
            Err(err) if err.is_full() => {
                pending = err.into_inner();
                std::thread::sleep(Duration::from_millis(4));
            }
            Err(_) => return, // приёмник закрыт
        }
    }
}

fn events_subscription() -> Subscription<Message> {
    iced::event::listen_with(|event, status, _window| match event {
        iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
            Some(Message::ModifiersChanged(modifiers))
        }
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. })
            if status == iced::event::Status::Ignored =>
        {
            Some(Message::KeyPressed(key, modifiers))
        }
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::CursorMoved(position))
        }
        // Кнопку могли отпустить, уведя курсор за пределы списка, — рамку
        // всё равно нужно завершить.
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(Message::PanelReleased)
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size))
        }
        _ => None,
    })
}

// ------------------------------------------------------------------ update

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::NavigateTo(path) => app.navigate(path),
        Message::Activate(index) => app.activate(index),

        Message::Back => {
            if app.history_pos > 0 {
                app.history_pos -= 1;
                let path = app.history[app.history_pos].clone();
                app.dir = path.clone();
                return app.reload(None);
            }
            Task::none()
        }
        Message::Forward => {
            if app.history_pos + 1 < app.history.len() {
                app.history_pos += 1;
                let path = app.history[app.history_pos].clone();
                app.dir = path.clone();
                return app.reload(None);
            }
            Task::none()
        }
        Message::Up => {
            let current = app.dir.clone();
            match current.parent() {
                Some(parent) => {
                    let parent = parent.to_path_buf();
                    let task = app.navigate(parent);
                    // Возврат наверх удобнее с выделенным исходным каталогом.
                    app.selection.select_only(current);
                    task
                }
                None => Task::none(),
            }
        }
        Message::Refresh => {
            app.quick = places::quick_access(app.config.lang, &app.config.bookmarks);
            app.drives = places::drives(app.config.lang);
            app.reload(None)
        }

        Message::Loaded(result) => {
            app.apply_load(result);
            Task::none()
        }

        Message::RowPressed(index) => {
            app.context = None;
            app.theme_menu = false;
            app.press_row(index);
            Task::none()
        }
        Message::RowHovered(index) => {
            app.hover = index;
            Task::none()
        }
        Message::RowRightPressed(index) => {
            if let Some(entry) = app.entry_at(index) {
                let path = entry.path.clone();
                // По правой кнопке выделяем элемент, только если он ещё не
                // входит в выделение — иначе групповое меню теряет смысл.
                if !app.selection.contains(&path) {
                    app.focus_index(index, false);
                }
                app.context = Some(Context {
                    position: app.cursor,
                    target: Some(path),
                });
            }
            Task::none()
        }
        Message::EmptyPressed => {
            app.context = None;
            app.theme_menu = false;
            app.selection.clear();
            Task::none()
        }
        Message::EmptyRightPressed => {
            app.marquee = None;
            app.selection.clear();
            app.context = Some(Context {
                position: app.cursor,
                target: None,
            });
            Task::none()
        }
        Message::SelectAll => {
            let paths: Vec<PathBuf> = app
                .filtered
                .iter()
                .filter_map(|&i| app.entries.get(i))
                .map(|e| e.path.clone())
                .collect();
            app.selection.select_all(paths);
            Task::none()
        }

        Message::PanelPressed => {
            app.context = None;
            app.theme_menu = false;

            // С Ctrl (или Shift) рамка дополняет выделение, иначе заменяет.
            let additive = app.modifiers.control() || app.modifiers.shift();
            let base: HashSet<PathBuf> = if additive {
                app.selection.iter().cloned().collect()
            } else {
                app.selection.clear();
                HashSet::new()
            };

            app.marquee = Some(Marquee {
                origin: None,
                current: Point::ORIGIN,
                base,
            });
            Task::none()
        }
        Message::PanelMoved(position) => {
            // Координаты приходят относительно видимой области, а рамка живёт
            // в координатах содержимого.
            let scroll = app.scroll;
            let Some(marquee) = app.marquee.as_mut() else {
                return Task::none();
            };

            let point = Point::new(position.x, position.y + scroll);
            if marquee.origin.is_none() {
                marquee.origin = Some(point);
            }
            marquee.current = point;

            app.apply_marquee();
            Task::none()
        }
        Message::PanelReleased => {
            app.marquee = None;
            Task::none()
        }
        Message::Scrolled(offset) => {
            app.scroll = offset;
            Task::none()
        }

        Message::CopySelection => {
            app.context = None;
            let paths = app.selected_paths();
            if !paths.is_empty() {
                app.clipboard = Some((Op::Copy, paths));
            }
            Task::none()
        }
        Message::CutSelection => {
            app.context = None;
            let paths = app.selected_paths();
            if !paths.is_empty() {
                app.clipboard = Some((Op::Move, paths));
            }
            Task::none()
        }
        Message::Paste => {
            app.context = None;
            let Some((op, sources)) = app.clipboard.clone() else {
                return Task::none();
            };
            if !ops::is_writable(&app.dir) {
                app.modal = Some(Modal::Error {
                    title: app.t(S::ErrPasteTitle).into(),
                    message: app
                        .config
                        .lang
                        .no_write_access(&app.dir.display().to_string()),
                });
                return Task::none();
            }
            if op == Op::Move {
                app.clipboard = None;
            }

            submit(Request::Submit {
                op,
                sources,
                dest: app.dir.clone(),
                conflict: app.config.conflict,
                naming: app.config.naming.clone(),
            })
        }

        Message::OpenSelected => {
            app.context = None;
            let paths = app.selected_paths();
            if let Some(first) = paths.first() {
                if first.is_dir() {
                    return app.navigate(first.clone());
                }
                for path in &paths {
                    if let Err(err) = ops::open_external(path) {
                        app.modal = Some(Modal::Error {
                            title: app.t(S::ErrOpenTitle).into(),
                            message: err.to_string(),
                        });
                        break;
                    }
                }
            }
            Task::none()
        }

        Message::RequestRename => {
            app.context = None;
            if let Some(path) = app.selected_paths().into_iter().next() {
                let value = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                app.modal = Some(Modal::Rename { path, value });
                return iced::widget::operation::focus(DIALOG_ID);
            }
            Task::none()
        }
        Message::RequestNewFolder => {
            app.context = None;
            app.modal = Some(Modal::NewFolder {
                value: app.t(S::NewFolderDefault).into(),
            });
            iced::widget::operation::focus(DIALOG_ID)
        }
        Message::RequestNewFile => {
            app.context = None;
            app.modal = Some(Modal::NewFile {
                value: app.t(S::NewFileDefault).into(),
            });
            iced::widget::operation::focus(DIALOG_ID)
        }
        Message::RequestDelete { permanent } => {
            app.context = None;
            let paths = app.selected_paths();
            if paths.is_empty() {
                return Task::none();
            }

            if permanent {
                app.modal = Some(Modal::ConfirmDelete {
                    paths,
                    permanent: true,
                });
                Task::none()
            } else {
                // Корзина обратима, подтверждение не нужно.
                app.selection.clear();
                delete_task(paths, false)
            }
        }
        Message::RequestProperties => {
            app.context = None;
            if let Some(path) = app.selected_paths().into_iter().next() {
                if let Ok(entry) = Entry::from_path(path) {
                    let contents = if entry.is_dir {
                        std::fs::read_dir(&entry.path).ok().map(|d| d.count())
                    } else {
                        None
                    };
                    app.modal = Some(Modal::Properties { entry, contents });
                }
            }
            Task::none()
        }
        Message::OperationDone(result) => {
            match result {
                Ok(select) => {
                    return app.reload(select);
                }
                Err(message) => {
                    app.modal = Some(Modal::Error {
                        title: app.t(S::ErrOperation).into(),
                        message,
                    });
                }
            }
            Task::none()
        }

        Message::ModalInput(value) => {
            match &mut app.modal {
                Some(Modal::Rename { value: v, .. })
                | Some(Modal::NewFolder { value: v })
                | Some(Modal::NewFile { value: v }) => *v = value,
                _ => {}
            }
            Task::none()
        }
        Message::ModalSubmit => app.submit_modal(),
        Message::ModalClose => {
            app.modal = None;
            Task::none()
        }
        Message::ContextClose => {
            app.context = None;
            Task::none()
        }
        Message::ShowJobs => {
            app.modal = Some(Modal::Jobs);
            Task::none()
        }

        Message::SetView(view) => {
            app.config.view = view;
            app.config.save();
            Task::none()
        }
        Message::SetSort(key) => {
            if app.config.sort_key == key {
                app.config.sort_ascending = !app.config.sort_ascending;
            } else {
                app.config.sort_key = key;
                app.config.sort_ascending = true;
            }
            app.config.save();
            app.resort();
            Task::none()
        }
        Message::SetTheme(choice) => {
            app.config.theme = choice;
            app.theme = choice.theme();
            app.theme_menu = false;
            app.config.save();
            Task::none()
        }
        Message::ToggleThemeMenu => {
            app.theme_menu = !app.theme_menu;
            app.context = None;
            Task::none()
        }
        Message::ToggleHidden => {
            app.config.show_hidden = !app.config.show_hidden;
            app.config.save();
            app.reload(None)
        }
        Message::ToggleFilter => {
            app.filter_visible = !app.filter_visible;
            if app.filter_visible {
                return iced::widget::operation::focus(FILTER_ID);
            }
            app.filter.clear();
            app.refilter();
            Task::none()
        }
        Message::FilterChanged(value) => {
            app.filter = value;
            app.refilter();
            Task::none()
        }
        Message::TogglePathEdit => {
            app.path_edit = match app.path_edit {
                Some(_) => None,
                None => Some(app.dir.display().to_string()),
            };
            if app.path_edit.is_some() {
                return iced::widget::operation::focus(PATH_ID);
            }
            Task::none()
        }
        Message::PathEditChanged(value) => {
            app.path_edit = Some(value);
            Task::none()
        }
        Message::PathEditSubmit => {
            let Some(raw) = app.path_edit.take() else {
                return Task::none();
            };
            let expanded = expand_path(&raw);
            if expanded.is_dir() {
                return app.navigate(expanded);
            }
            app.modal = Some(Modal::Error {
                title: app.t(S::ErrPathNotFound).into(),
                message: raw,
            });
            Task::none()
        }

        Message::PlaceHovered(index) => {
            app.hover_place = index;
            Task::none()
        }
        Message::ToggleBookmark => {
            let dir = app.dir.clone();
            if let Some(pos) = app.config.bookmarks.iter().position(|p| *p == dir) {
                app.config.bookmarks.remove(pos);
            } else {
                app.config.bookmarks.push(dir);
            }
            app.config.save();
            app.quick = places::quick_access(app.config.lang, &app.config.bookmarks);
            Task::none()
        }

        Message::Jobs(event) => app.handle_job_event(event),
        Message::CancelJob(id) => submit(Request::Cancel(id)),
        Message::PauseJob(id) => submit(Request::Pause(id)),
        Message::ResumeJob(id) => submit(Request::Resume(id)),
        Message::DismissJob(id) => submit(Request::Dismiss(id)),
        Message::DismissFinished => submit(Request::DismissFinished),

        Message::ModifiersChanged(modifiers) => {
            app.modifiers = modifiers;
            Task::none()
        }
        Message::CursorMoved(position) => {
            app.cursor = position;
            Task::none()
        }
        Message::WindowResized(size) => {
            app.window = size;
            app.config.window_size = (size.width, size.height);
            Task::none()
        }
        Message::KeyPressed(key, modifiers) => app.handle_key(key, modifiers),
        Message::RefreshDrives => {
            app.drives = places::drives(app.config.lang);
            app.usage = ipc::disk_usage(&app.dir);
            Task::none()
        }
        Message::Noop => Task::none(),
    }
}

// -------------------------------------------------------------------- view

pub fn view(app: &App) -> Element<'_, Message> {
    let body = iced::widget::column![
        toolbar::view(app),
        iced::widget::row![sidebar::view(app), panel::view(app)].height(Fill),
        statusbar::view(app),
    ];

    let base = container(body)
        .width(Fill)
        .height(Fill)
        .style(style::panel);

    let mut layers = stack![base];

    if app.theme_menu {
        layers = layers.push(dialogs::theme_menu(app));
    }

    if let Some(context) = &app.context {
        layers = layers.push(dialogs::context_menu(app, context));
    }

    if let Some(modal) = &app.modal {
        layers = layers.push(opaque(
            mouse_area(
                container(dialogs::modal(app, modal))
                    .width(Fill)
                    .height(Fill)
                    .center_x(Fill)
                    .center_y(Fill)
                    .style(style::backdrop),
            )
            .on_press(Message::ModalClose),
        ));
    }

    layers.into()
}

// ------------------------------------------------------------------ помощь

impl App {
    pub fn lang(&self) -> Lang {
        self.config.lang
    }

    /// Короткий доступ к переводу строки.
    pub fn t(&self, key: S) -> &'static str {
        self.config.lang.s(key)
    }

    pub fn entry_at(&self, index: usize) -> Option<&Entry> {
        self.filtered.get(index).and_then(|&i| self.entries.get(i))
    }

    pub fn visible_entries(&self) -> impl Iterator<Item = (usize, &Entry)> {
        self.filtered
            .iter()
            .enumerate()
            .filter_map(|(pos, &i)| self.entries.get(i).map(|e| (pos, e)))
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        // Порядок как в списке — предсказуемее для пользователя.
        let mut paths: Vec<PathBuf> = self
            .visible_entries()
            .filter(|(_, e)| self.selection.contains(&e.path))
            .map(|(_, e)| e.path.clone())
            .collect();

        if paths.is_empty() {
            paths = self.selection.iter().cloned().collect();
        }
        paths
    }

    pub fn is_bookmarked(&self) -> bool {
        self.config.bookmarks.contains(&self.dir)
    }

    pub fn active_jobs(&self) -> impl Iterator<Item = &Job> {
        self.jobs.iter().filter(|job| job.state.is_active())
    }

    fn navigate(&mut self, path: PathBuf) -> Task<Message> {
        if path == self.dir {
            return self.reload(None);
        }

        self.history.truncate(self.history_pos + 1);
        self.history.push(path.clone());
        self.history_pos = self.history.len() - 1;
        self.dir = path;

        self.reload(None)
    }

    fn reload(&mut self, select: Option<PathBuf>) -> Task<Message> {
        self.loading = true;
        self.context = None;
        self.marquee = None;
        self.filter.clear();
        self.filter_visible = false;
        self.path_edit = None;

        self.config.last_dir = Some(self.dir.clone());
        self.config.save();

        // Новый каталог показываем с начала списка.
        self.scroll = 0.0;
        Task::batch([
            iced::widget::operation::scroll_to(
                LIST_ID,
                iced::widget::operation::AbsoluteOffset { x: 0.0, y: 0.0 },
            ),
            load(self.dir.clone(), self.config.show_hidden, select),
        ])
    }

    fn apply_load(&mut self, result: LoadResult) {
        if result.path != self.dir {
            return; // ответ от устаревшего запроса
        }

        self.entries = result.entries;
        self.read_error = result.error;
        self.usage = result.usage;
        self.loading = false;
        self.hover = None;

        self.resort();

        // Оставляем выделенным только то, что всё ещё существует.
        let alive: HashSet<PathBuf> = self.entries.iter().map(|e| e.path.clone()).collect();
        self.selection.retain_existing(&alive);

        if let Some(path) = result.select {
            if alive.contains(&path) {
                self.selection.select_only(path);
            }
        }
    }

    fn resort(&mut self) {
        entry::sort(
            &mut self.entries,
            self.config.sort_key,
            self.config.sort_ascending,
            self.config.dirs_first,
        );
        self.refilter();
    }

    fn refilter(&mut self) {
        let needle = self.filter.to_lowercase();

        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| needle.is_empty() || e.name.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
    }

    fn press_row(&mut self, index: usize) {
        let ctrl = self.modifiers.control();
        let shift = self.modifiers.shift();

        let Self {
            selection,
            entries,
            filtered,
            ..
        } = self;

        selection.click(index, ctrl, shift, |i| path_at(entries, filtered, i));
    }

    /// Пересчитать выделение под текущую рамку.
    fn apply_marquee(&mut self) {
        let Some(marquee) = self.marquee.take() else {
            return;
        };

        if let Some(area) = marquee.area() {
            let hits = panel::Geometry::of(self).hits(area, self.filtered.len());

            let Self {
                selection,
                entries,
                filtered,
                ..
            } = self;

            selection.apply_marquee(&marquee.base, &hits, |i| path_at(entries, filtered, i));
        }

        self.marquee = Some(marquee);
    }

    fn activate(&mut self, index: usize) -> Task<Message> {
        let Some(entry) = self.entry_at(index) else {
            return Task::none();
        };

        if entry.is_dir {
            let path = entry.path.clone();
            return self.navigate(path);
        }

        if let Err(err) = ops::open_external(&entry.path) {
            self.modal = Some(Modal::Error {
                title: self.config.lang.s(S::ErrOpenTitle).into(),
                message: err.to_string(),
            });
        }
        Task::none()
    }

    fn submit_modal(&mut self) -> Task<Message> {
        let Some(modal) = self.modal.take() else {
            return Task::none();
        };

        match modal {
            Modal::Rename { path, value } => Task::perform(
                async move {
                    ops::rename(&path, &value)
                        .map(Some)
                        .map_err(|e| e.to_string())
                },
                Message::OperationDone,
            ),
            Modal::NewFolder { value } => {
                let dir = self.dir.clone();
                Task::perform(
                    async move {
                        ops::create_dir(&dir, &value)
                            .map(Some)
                            .map_err(|e| e.to_string())
                    },
                    Message::OperationDone,
                )
            }
            Modal::NewFile { value } => {
                let dir = self.dir.clone();
                Task::perform(
                    async move {
                        ops::create_file(&dir, &value)
                            .map(Some)
                            .map_err(|e| e.to_string())
                    },
                    Message::OperationDone,
                )
            }
            Modal::ConfirmDelete { paths, permanent } => {
                self.selection.clear();
                delete_task(paths, permanent)
            }
            _ => Task::none(),
        }
    }

    fn handle_job_event(&mut self, event: JobEvent) -> Task<Message> {
        match event {
            JobEvent::Connected => self.daemon_online = true,
            JobEvent::Disconnected => self.daemon_online = false,
            JobEvent::Message(Response::Snapshot { jobs }) => {
                self.daemon_online = true;
                self.jobs = jobs;
            }
            JobEvent::Message(Response::Updated { job }) => {
                let was_active = self
                    .jobs
                    .iter()
                    .find(|j| j.id == job.id)
                    .map(|j| j.state.is_active())
                    .unwrap_or(true);
                let finished_now = was_active && !job.state.is_active();

                // Задача могла менять как раз тот каталог, который открыт.
                let touches_view = job.dest == self.dir
                    || job
                        .sources
                        .iter()
                        .any(|s| s.parent() == Some(self.dir.as_path()));

                match self.jobs.iter_mut().find(|j| j.id == job.id) {
                    Some(slot) => *slot = job,
                    None => self.jobs.push(job),
                }

                if finished_now && touches_view {
                    return load(self.dir.clone(), self.config.show_hidden, None);
                }
            }
            JobEvent::Message(Response::Removed { id }) => {
                self.jobs.retain(|job| job.id != id);
            }
            JobEvent::Message(Response::Accepted { .. }) => {}
            JobEvent::Message(Response::Error { message }) => {
                self.modal = Some(Modal::Error {
                    title: self.config.lang.s(S::ErrJob).into(),
                    message,
                });
            }
        }

        Task::none()
    }

    fn handle_key(&mut self, key: Key, modifiers: Modifiers) -> Task<Message> {
        // Пока открыт диалог, работают только Enter и Escape.
        if self.modal.is_some() {
            return match key {
                Key::Named(Named::Escape) => {
                    self.modal = None;
                    Task::none()
                }
                Key::Named(Named::Enter) => self.submit_modal(),
                _ => Task::none(),
            };
        }

        if self.context.is_some() || self.theme_menu {
            if let Key::Named(Named::Escape) = key {
                self.context = None;
                self.theme_menu = false;
                return Task::none();
            }
        }

        match key {
            Key::Named(Named::Escape) => {
                self.marquee = None;
                if self.filter_visible {
                    self.filter_visible = false;
                    self.filter.clear();
                    self.refilter();
                } else {
                    self.selection.clear();
                }
                Task::none()
            }
            Key::Named(Named::Enter) => update(self, Message::OpenSelected),
            Key::Named(Named::Backspace) => update(self, Message::Up),
            Key::Named(Named::F5) => update(self, Message::Refresh),
            Key::Named(Named::F2) => update(self, Message::RequestRename),
            Key::Named(Named::Delete) => update(
                self,
                Message::RequestDelete {
                    permanent: modifiers.shift(),
                },
            ),
            Key::Named(Named::ArrowUp) => self.move_cursor(-1, modifiers.shift()),
            Key::Named(Named::ArrowDown) => self.move_cursor(1, modifiers.shift()),
            Key::Named(Named::ArrowLeft) if modifiers.alt() => update(self, Message::Back),
            Key::Named(Named::ArrowRight) if modifiers.alt() => update(self, Message::Forward),
            Key::Named(Named::Home) if !self.filtered.is_empty() => {
                self.select_index(0, modifiers.shift())
            }
            Key::Named(Named::End) if !self.filtered.is_empty() => {
                self.select_index(self.filtered.len() - 1, modifiers.shift())
            }
            Key::Character(c) if modifiers.control() => match c.as_str() {
                "c" => update(self, Message::CopySelection),
                "x" => update(self, Message::CutSelection),
                "v" => update(self, Message::Paste),
                "a" => update(self, Message::SelectAll),
                "h" => update(self, Message::ToggleHidden),
                "f" => update(self, Message::ToggleFilter),
                "l" => update(self, Message::TogglePathEdit),
                "d" => update(self, Message::ToggleBookmark),
                "n" => update(self, Message::RequestNewFolder),
                "1" => update(self, Message::SetView(ViewMode::Details)),
                "2" => update(self, Message::SetView(ViewMode::Compact)),
                "3" => update(self, Message::SetView(ViewMode::Icons)),
                _ => Task::none(),
            },
            _ => Task::none(),
        }
    }

    fn move_cursor(&mut self, delta: isize, extend: bool) -> Task<Message> {
        if self.filtered.is_empty() {
            return Task::none();
        }

        let last = self.filtered.len() as isize - 1;
        let current = self.selection.focus().map(|f| f as isize).unwrap_or(-1);
        let next = (current + delta).clamp(0, last) as usize;

        self.select_index(next, extend)
    }

    fn select_index(&mut self, index: usize, extend: bool) -> Task<Message> {
        self.focus_index(index, extend);
        self.scroll_into_view(index)
    }

    /// Выделить элемент без прокрутки списка.
    fn focus_index(&mut self, index: usize, extend: bool) {
        if index >= self.filtered.len() {
            return;
        }

        let Self {
            selection,
            entries,
            filtered,
            ..
        } = self;

        selection.move_focus(index, extend, |i| path_at(entries, filtered, i));
    }

    /// Довернуть список так, чтобы элемент оказался в видимой области.
    fn scroll_into_view(&self, index: usize) -> Task<Message> {
        let cell = panel::Geometry::of(self).rect(index);
        let viewport = self.list_height();

        let top = (cell.y - panel::PAD_Y).max(0.0);
        let bottom = cell.y + cell.height + panel::PAD_Y;

        let offset = if top < self.scroll {
            top
        } else if bottom > self.scroll + viewport {
            (bottom - viewport).max(0.0)
        } else {
            return Task::none();
        };

        iced::widget::operation::scroll_to(
            LIST_ID,
            iced::widget::operation::AbsoluteOffset { x: 0.0, y: offset },
        )
    }

    /// Высота видимой части списка — панели инструментов и статуса вычитаются.
    fn list_height(&self) -> f32 {
        let mut height = self.window.height - toolbar::HEIGHT - STATUS_HEIGHT;
        if self.filter_visible {
            height -= FILTER_HEIGHT;
        }
        if self.config.view == ViewMode::Details {
            height -= HEADER_HEIGHT;
        }
        height.max(120.0)
    }
}

/// Путь видимого элемента по его порядковому номеру.
fn path_at(entries: &[Entry], filtered: &[usize], index: usize) -> Option<PathBuf> {
    filtered
        .get(index)
        .and_then(|&i| entries.get(i))
        .map(|entry| entry.path.clone())
}

// ------------------------------------------------------------------ задачи

fn load(path: PathBuf, show_hidden: bool, select: Option<PathBuf>) -> Task<Message> {
    Task::perform(
        async move {
            let usage = ipc::disk_usage(&path);
            match entry::read_dir(&path, show_hidden) {
                Ok(entries) => LoadResult {
                    path,
                    entries,
                    error: None,
                    usage,
                    select,
                },
                Err(err) => LoadResult {
                    path,
                    entries: Vec::new(),
                    error: Some(err.to_string()),
                    usage,
                    select,
                },
            }
        },
        Message::Loaded,
    )
}

fn submit(request: Request) -> Task<Message> {
    Task::perform(
        async move { ipc::request(&request, false).err().map(|e| e.to_string()) },
        |error| match error {
            Some(message) => Message::OperationDone(Err(message)),
            None => Message::Noop,
        },
    )
}

fn delete_task(paths: Vec<PathBuf>, permanent: bool) -> Task<Message> {
    Task::perform(
        async move {
            let mut failures = Vec::new();

            for path in &paths {
                let result = if permanent {
                    ops::delete(path)
                } else {
                    ops::move_to_trash(path)
                };
                if let Err(err) = result {
                    failures.push(format!("{}: {err}", path.display()));
                }
            }

            if failures.is_empty() {
                Ok(None)
            } else {
                Err(failures.join("\n"))
            }
        },
        Message::OperationDone,
    )
}

fn expand_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed == "~" {
        return places::home();
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return places::home().join(rest);
    }
    PathBuf::from(trimmed)
}

/// Контекстное меню прижимается к курсору, но не вылезает за окно.
pub fn clamp_menu(app: &App, position: Point, size: Size) -> Point {
    Point::new(
        position
            .x
            .min((app.window.width - size.width - 8.0).max(8.0)),
        position
            .y
            .min((app.window.height - size.height - 8.0).max(8.0)),
    )
}

pub fn pinned<'a>(element: Element<'a, Message>, position: Point) -> Element<'a, Message> {
    pin(element).x(position.x).y(position.y).into()
}
