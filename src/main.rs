//! Файловый менеджер для Wayland без зависимостей от GTK и Qt.
//!
//! Один бинарник работает в двух режимах:
//!   * без аргументов — графический интерфейс (iced/wgpu, wayland);
//!   * `--copy-daemon` — фоновая служба копирования, переживающая закрытие окна.

mod config;
mod fsops;
mod i18n;
mod ipc;
mod jobs;
mod ui;

use std::path::PathBuf;

use iced::window;

const USAGE: &str = "\
Файловый менеджер (Wayland, без GTK/Qt)

Использование:
    file-man [ПАПКА]        открыть окно менеджера
    file-man --copy-daemon  запустить службу фонового копирования
    file-man --help         показать эту справку
    file-man --version      показать версию

Служба копирования поднимается автоматически при первой операции и
продолжает работу после закрытия окна: новый экземпляр менеджера
подключается к ней и показывает актуальный прогресс.
";

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Режим демона: сюда нас запускает GUI, когда служба ещё не поднята.
    if args.iter().any(|arg| arg == ipc::DAEMON_ARG) {
        if let Err(err) = jobs::daemon::run() {
            eprintln!("file-man: демон копирования остановлен: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }

    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print!("{USAGE}");
        return Ok(());
    }

    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        println!("file-man {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Первый позиционный аргумент — каталог для открытия (в том числе file://).
    if let Some(target) = args.iter().find(|arg| !arg.starts_with('-')) {
        if let Some(path) = resolve_target(target) {
            ui::set_start_dir(path);
        }
    }

    let saved = config::Config::load();

    let settings = window::Settings {
        min_size: Some(iced::Size::new(720.0, 460.0)),
        ..window::Settings::default()
    };

    iced::application(ui::boot, ui::update, ui::view)
        .title(ui::title)
        .theme(ui::theme)
        .subscription(ui::subscription)
        .window(settings)
        .window_size(iced::Size::new(
            saved.window_size.0.max(720.0),
            saved.window_size.1.max(460.0),
        ))
        .antialiasing(true)
        .run()
}

/// Аргумент может быть путём, `file://`-ссылкой или файлом — тогда открываем
/// содержащую его папку.
fn resolve_target(raw: &str) -> Option<PathBuf> {
    let path = match raw.strip_prefix("file://") {
        Some(rest) => PathBuf::from(fsops::places::percent_decode(rest)),
        None => PathBuf::from(raw),
    };

    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().ok()?.join(path)
    };

    if path.is_dir() {
        Some(path)
    } else {
        path.parent().filter(|p| p.is_dir()).map(PathBuf::from)
    }
}
