//! Локализация интерфейса.
//!
//! Строки хранятся одной таблицей: ключ и по переводу на каждый язык. Язык
//! задаётся в конфиге (`"lang": "En" | "Ru"`), по умолчанию английский.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    En,
    Ru,
}

/// Объявляет ключи строк и переводы: `Ключ => "english" / "русский"`.
macro_rules! strings {
    ($($name:ident => $en:literal / $ru:literal,)*) => {
        /// Ключ строки интерфейса.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum S {
            $($name,)*
        }

        impl Lang {
            /// Перевод строки на текущий язык.
            pub fn s(self, key: S) -> &'static str {
                match key {
                    $(S::$name => match self {
                        Lang::En => $en,
                        Lang::Ru => $ru,
                    },)*
                }
            }
        }
    };
}

strings! {
    // окно и панель инструментов
    AppName          => "File Manager" / "Файловый менеджер",
    Back             => "Back (Alt+←)" / "Назад (Alt+←)",
    Forward          => "Forward (Alt+→)" / "Вперёд (Alt+→)",
    Up               => "Up (Backspace)" / "Наверх (Backspace)",
    Refresh          => "Refresh (F5)" / "Обновить (F5)",
    NewFolderHint    => "New folder (Ctrl+N)" / "Создать папку (Ctrl+N)",
    BookmarkAdd      => "Add to quick access (Ctrl+D)" / "В быстрый доступ (Ctrl+D)",
    BookmarkRemove   => "Remove from quick access (Ctrl+D)" / "Убрать из быстрого доступа (Ctrl+D)",
    FilterHint       => "Filter by name (Ctrl+F)" / "Фильтр по имени (Ctrl+F)",
    HiddenHint       => "Hidden files (Ctrl+H)" / "Скрытые файлы (Ctrl+H)",
    ThemeHint        => "Theme" / "Тема оформления",
    ThemeTitle       => "Theme" / "Оформление",
    PathPlaceholder  => "/path/to/folder" / "/путь/к/папке",
    FilterPlaceholder=> "Filter by name…" / "Фильтр по имени…",

    // режимы отображения
    ViewDetails      => "Details" / "Подробно",
    ViewCompact      => "Compact" / "Компактно",
    ViewIcons        => "Icons" / "Значки",

    // колонки и сортировка
    ColumnName       => "Name" / "Имя",
    ColumnSize       => "Size" / "Размер",
    ColumnKind       => "Type" / "Тип",
    ColumnModified   => "Modified" / "Изменён",

    // типы файлов
    KindFolder       => "Folder" / "Папка",
    KindImage        => "Image" / "Изображение",
    KindVideo        => "Video" / "Видео",
    KindAudio        => "Audio" / "Аудио",
    KindArchive      => "Archive" / "Архив",
    KindCode         => "Source code" / "Исходный код",
    KindDocument     => "Document" / "Документ",
    KindPdf          => "PDF" / "PDF",
    KindText         => "Text" / "Текст",
    KindExecutable   => "Program" / "Программа",
    KindUnknown      => "File" / "Файл",

    // левая панель
    QuickAccess      => "Quick access" / "Быстрый доступ",
    Devices          => "Devices" / "Устройства",
    PlaceHome        => "Home" / "Домашняя папка",
    PlaceDesktop     => "Desktop" / "Рабочий стол",
    PlaceDownloads   => "Downloads" / "Загрузки",
    PlaceDocuments   => "Documents" / "Документы",
    PlacePictures    => "Pictures" / "Изображения",
    PlaceMusic       => "Music" / "Музыка",
    PlaceVideos      => "Videos" / "Видео",
    PlaceTrash       => "Trash" / "Корзина",
    PlaceRoot        => "File system" / "Файловая система",

    // основная панель
    DirUnavailable   => "Folder unavailable" / "Каталог недоступен",
    EmptyTitle       => "Nothing here" / "Здесь пусто",
    EmptyHint        => "This folder has no files" / "В этой папке нет файлов",
    NoMatchTitle     => "Nothing found" / "Ничего не найдено",
    NoMatchHint      => "Try a different filter" / "Измените условие фильтра",

    // статусная строка
    StatusEmpty      => "empty" / "пусто",
    Loading          => "refreshing…" / "обновление…",
    JobScanning      => "counting…" / "подсчёт объёма…",
    JobPaused        => "paused" / "пауза",
    JobRunning       => "running" / "выполняется",
    JobDone          => "done" / "готово",
    JobCancelled     => "cancelled" / "отменено",
    JobFailed        => "failed" / "ошибка",
    JobDoneNotice    => "Copy finished" / "Копирование завершено",
    JobFailedNotice  => "Task finished with an error" / "Задача завершилась с ошибкой",
    JobCancelNotice  => "Task cancelled" / "Задача отменена",
    DaemonOffline    => "copy service unavailable" / "служба копирования недоступна",
    Resume           => "Resume" / "Продолжить",
    Pause            => "Pause" / "Пауза",
    Cancel           => "Cancel" / "Отменить",
    Hide             => "Hide" / "Скрыть",

    // контекстное меню
    Open             => "Open" / "Открыть",
    Copy             => "Copy" / "Копировать",
    Cut              => "Cut" / "Вырезать",
    Paste            => "Paste" / "Вставить",
    Rename           => "Rename" / "Переименовать",
    MoveToTrash      => "Move to trash" / "В корзину",
    DeleteForever    => "Delete permanently" / "Удалить навсегда",
    NewFolder        => "New folder" / "Создать папку",
    NewFile          => "New file" / "Создать файл",
    Properties       => "Properties" / "Свойства",

    // диалоги
    NewNameLabel     => "New name" / "Новое имя",
    FolderNameLabel  => "Folder name" / "Имя папки",
    FileNameLabel    => "File name" / "Имя файла",
    NewFolderDefault => "New folder" / "Новая папка",
    NewFileDefault   => "New file" / "Новый файл",
    DialogCancel     => "Cancel" / "Отмена",
    DialogApply      => "Apply" / "Готово",
    DialogClose      => "Close" / "Закрыть",
    DialogOk         => "Got it" / "Понятно",
    DeleteTitle      => "Delete permanently?" / "Удалить безвозвратно?",
    TrashTitle       => "Move to trash?" / "Переместить в корзину?",
    DeleteWarning    => "These items cannot be restored." / "Восстановить эти объекты будет невозможно.",
    TrashWarning     => "Items can be restored from the trash." / "Объекты можно будет вернуть из корзины.",
    DeleteConfirm    => "Delete" / "Удалить",
    FieldName        => "Name" / "Имя",
    FieldPath        => "Path" / "Путь",
    FieldKind        => "Type" / "Тип",
    FieldContents    => "Contents" / "Содержимое",
    FieldSize        => "Size" / "Размер",
    FieldModified    => "Modified" / "Изменён",
    FieldHidden      => "Hidden" / "Скрытый",
    FieldLinkTarget  => "Points to" / "Ссылка на",
    Yes              => "yes" / "да",
    JobsTitle        => "Background tasks" / "Фоновые задачи",
    JobsEmpty        => "No background tasks" / "Фоновых задач нет",
    JobsClearDone    => "Clear finished" / "Убрать завершённые",

    // ошибки
    ErrPasteTitle    => "Cannot paste" / "Не удалось вставить",
    ErrOpenTitle     => "Cannot open" / "Не удалось открыть",
    ErrOperation     => "Operation failed" / "Операция не выполнена",
    ErrPathNotFound  => "Path not found" / "Путь не найден",
    ErrJob           => "Background task error" / "Ошибка фоновой задачи",

    // операции над файлами
    OpCopy           => "Copying" / "Копирование",
    OpMove           => "Moving" / "Перемещение",
}

impl Lang {
    /// Заголовок окна: «catnip — File Manager».
    pub fn window_title(self, folder: &str) -> String {
        format!("{folder} — {}", self.s(S::AppName))
    }

    /// «12 matches» / «12 совпадений».
    pub fn matches(self, count: usize) -> String {
        match self {
            Lang::En => format!("{count} {}", if count == 1 { "match" } else { "matches" }),
            Lang::Ru => format!("{count} {}", plural_ru(count, "совпадение", "совпадения", "совпадений")),
        }
    }

    /// «3 folders» / «3 папки».
    pub fn folders(self, count: usize) -> String {
        match self {
            Lang::En => format!("{count} {}", if count == 1 { "folder" } else { "folders" }),
            Lang::Ru => format!("{count} {}", plural_ru(count, "папка", "папки", "папок")),
        }
    }

    /// «7 files» / «7 файлов».
    pub fn files(self, count: usize) -> String {
        match self {
            Lang::En => format!("{count} {}", if count == 1 { "file" } else { "files" }),
            Lang::Ru => format!("{count} {}", plural_ru(count, "файл", "файла", "файлов")),
        }
    }

    /// «124 items» / «124 объекта».
    pub fn items(self, count: u64) -> String {
        match self {
            Lang::En => format!("{count} {}", if count == 1 { "item" } else { "items" }),
            Lang::Ru => format!(
                "{count} {}",
                plural_ru(count as usize, "объект", "объекта", "объектов")
            ),
        }
    }

    /// «selected 3 (1.2 GB)» / «выбрано 3 (1,2 ГБ)».
    pub fn selected(self, count: usize, bytes: u64) -> String {
        let head = match self {
            Lang::En => format!("selected {count}"),
            Lang::Ru => format!("выбрано {count}"),
        };
        if bytes == 0 {
            head
        } else {
            format!("{head} ({})", self.size(bytes))
        }
    }

    /// «546 GB free» / «546 ГБ свободно».
    pub fn free_space(self, bytes: u64) -> String {
        match self {
            Lang::En => format!("{} free", self.size(bytes)),
            Lang::Ru => format!("{} свободно", self.size(bytes)),
        }
    }

    /// То же, но для статусной строки: «· 546 GB free».
    pub fn free_space_inline(self, bytes: u64) -> String {
        format!("· {}", self.free_space(bytes))
    }

    /// «1.4 GB» / «1,4 ГБ».
    pub fn size(self, bytes: u64) -> String {
        const EN: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        const RU: [&str; 5] = ["Б", "КБ", "МБ", "ГБ", "ТБ"];

        let units = match self {
            Lang::En => EN,
            Lang::Ru => RU,
        };

        if bytes < 1024 {
            return format!("{bytes} {}", units[0]);
        }

        let mut value = bytes as f64;
        let mut unit = 0;
        while value >= 1024.0 && unit < units.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }

        let text = if value < 10.0 {
            format!("{value:.1}")
        } else {
            format!("{value:.0}")
        };

        let text = match self {
            Lang::En => text,
            Lang::Ru => text.replace('.', ","),
        };

        format!("{text} {}", units[unit])
    }

    /// «2 min 5 s» / «2 мин 5 с».
    pub fn duration(self, secs: u64) -> String {
        match self {
            Lang::En => {
                if secs < 60 {
                    format!("{secs} s")
                } else if secs < 3600 {
                    format!("{} min {} s", secs / 60, secs % 60)
                } else {
                    format!("{} h {} min", secs / 3600, (secs % 3600) / 60)
                }
            }
            Lang::Ru => {
                if secs < 60 {
                    format!("{secs} с")
                } else if secs < 3600 {
                    format!("{} мин {} с", secs / 60, secs % 60)
                } else {
                    format!("{} ч {} мин", secs / 3600, (secs % 3600) / 60)
                }
            }
        }
    }

    /// «45% · 320 MB/s · 12 s left».
    pub fn speed(self, bytes_per_sec: u64) -> String {
        match self {
            Lang::En => format!("{}/s", self.size(bytes_per_sec)),
            Lang::Ru => format!("{}/с", self.size(bytes_per_sec)),
        }
    }

    pub fn eta(self, secs: u64) -> String {
        match self {
            Lang::En => format!("{} left", self.duration(secs)),
            Lang::Ru => format!("осталось {}", self.duration(secs)),
        }
    }

    /// «12 errors» / «Ошибок: 12».
    pub fn errors(self, count: usize) -> String {
        match self {
            Lang::En => format!("{count} {}", if count == 1 { "error" } else { "errors" }),
            Lang::Ru => format!("Ошибок: {count}"),
        }
    }

    /// «…and 5 more» / «…и ещё 5».
    pub fn and_more(self, count: usize) -> String {
        match self {
            Lang::En => format!("…and {count} more"),
            Lang::Ru => format!("…и ещё {count}"),
        }
    }

    /// «2.1 GB of 14 GB · 7 of 206 items».
    pub fn job_amounts(self, done: u64, total: u64, done_items: u64, total_items: u64) -> String {
        match self {
            Lang::En => format!(
                "{} of {} · {done_items} of {total_items} items",
                self.size(done),
                self.size(total)
            ),
            Lang::Ru => format!(
                "{} из {} · {done_items} из {total_items} объектов",
                self.size(done),
                self.size(total)
            ),
        }
    }

    /// «No write access to “/srv”».
    pub fn no_write_access(self, path: &str) -> String {
        match self {
            Lang::En => format!("No write permission for “{path}”"),
            Lang::Ru => format!("Нет прав на запись в «{path}»"),
        }
    }
}

/// Русские числительные: 1 файл, 2 файла, 5 файлов.
fn plural_ru<'a>(count: usize, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let n = count % 100;
    if (11..=14).contains(&n) {
        return many;
    }
    match n % 10 {
        1 => one,
        2..=4 => few,
        _ => many,
    }
}
