//! Модель элемента каталога, чтение и сортировка.

use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Тип содержимого — нужен для выбора иконки и сортировки по типу.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Folder,
    Image,
    Video,
    Audio,
    Archive,
    Code,
    Document,
    Pdf,
    Text,
    Executable,
    Unknown,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Folder => "Папка",
            Kind::Image => "Изображение",
            Kind::Video => "Видео",
            Kind::Audio => "Аудио",
            Kind::Archive => "Архив",
            Kind::Code => "Исходный код",
            Kind::Document => "Документ",
            Kind::Pdf => "PDF",
            Kind::Text => "Текст",
            Kind::Executable => "Программа",
            Kind::Unknown => "Файл",
        }
    }

    fn from_extension(ext: &str) -> Self {
        const IMAGE: &[&str] = &[
            "png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico", "tiff", "tif", "avif",
            "heic", "jxl", "xcf", "psd", "raw", "cr2", "nef",
        ];
        const VIDEO: &[&str] = &[
            "mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "m4v", "mpg", "mpeg", "ts", "vob",
            "3gp", "ogv",
        ];
        const AUDIO: &[&str] = &[
            "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma", "aiff", "mid", "midi",
        ];
        const ARCHIVE: &[&str] = &[
            "zip", "tar", "gz", "bz2", "xz", "zst", "7z", "rar", "tgz", "txz", "tbz", "lz4",
            "iso", "img", "deb", "rpm", "pkg", "apk", "jar", "cab",
        ];
        const CODE: &[&str] = &[
            "rs", "c", "h", "cpp", "hpp", "cc", "cxx", "py", "js", "ts", "tsx", "jsx", "go",
            "java", "kt", "rb", "php", "sh", "bash", "zsh", "fish", "lua", "pl", "swift", "cs",
            "scala", "hs", "ml", "ex", "exs", "zig", "nim", "dart", "vim", "sql", "json", "yaml",
            "yml", "toml", "xml", "html", "htm", "css", "scss", "less", "ini", "conf", "cfg",
            "makefile", "cmake", "nix", "patch", "diff",
        ];
        const DOCUMENT: &[&str] = &[
            "doc", "docx", "odt", "rtf", "xls", "xlsx", "ods", "csv", "ppt", "pptx", "odp",
            "epub", "mobi", "djvu",
        ];
        const TEXT: &[&str] = &["txt", "md", "log", "nfo", "srt", "sub", "tex", "org", "rst"];
        const EXEC: &[&str] = &["appimage", "run", "bin", "exe", "so", "o", "a", "elf"];

        let ext = ext.to_ascii_lowercase();
        let ext = ext.as_str();

        if IMAGE.contains(&ext) {
            Kind::Image
        } else if VIDEO.contains(&ext) {
            Kind::Video
        } else if AUDIO.contains(&ext) {
            Kind::Audio
        } else if ARCHIVE.contains(&ext) {
            Kind::Archive
        } else if ext == "pdf" {
            Kind::Pdf
        } else if CODE.contains(&ext) {
            Kind::Code
        } else if DOCUMENT.contains(&ext) {
            Kind::Document
        } else if TEXT.contains(&ext) {
            Kind::Text
        } else if EXEC.contains(&ext) {
            Kind::Executable
        } else {
            Kind::Unknown
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    /// Битая ссылка: цель не существует.
    pub is_broken_link: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub kind: Kind,
    pub hidden: bool,
}

impl Entry {
    pub fn from_path(path: PathBuf) -> io::Result<Self> {
        let link_meta = fs::symlink_metadata(&path)?;
        let is_symlink = link_meta.file_type().is_symlink();

        // У ссылки интересует цель: каталог это или файл.
        let meta = if is_symlink {
            fs::metadata(&path).ok()
        } else {
            Some(link_meta.clone())
        };

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            meta.as_ref()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        };

        let kind = if is_dir {
            Kind::Folder
        } else {
            let by_ext = path
                .extension()
                .map(|e| Kind::from_extension(&e.to_string_lossy()))
                .unwrap_or(Kind::Unknown);
            if by_ext == Kind::Unknown && executable {
                Kind::Executable
            } else {
                by_ext
            }
        };

        Ok(Self {
            hidden: name.starts_with('.'),
            name,
            is_dir,
            is_symlink,
            is_broken_link: is_symlink && meta.is_none(),
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            modified: meta.as_ref().and_then(|m| m.modified().ok()),
            kind,
            path,
        })
    }
}

/// Прочитать каталог. Ошибки по отдельным элементам молча пропускаются.
pub fn read_dir(dir: &Path, show_hidden: bool) -> io::Result<Vec<Entry>> {
    let mut out = Vec::new();

    for entry in fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();

        let hidden = entry
            .file_name()
            .to_string_lossy()
            .starts_with('.');
        if hidden && !show_hidden {
            continue;
        }

        if let Ok(item) = Entry::from_path(path) {
            out.push(item);
        }
    }

    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortKey {
    Name,
    Size,
    Modified,
    Kind,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Name => "Имя",
            SortKey::Size => "Размер",
            SortKey::Modified => "Изменён",
            SortKey::Kind => "Тип",
        }
    }
}

pub fn sort(entries: &mut [Entry], key: SortKey, ascending: bool, dirs_first: bool) {
    entries.sort_by(|a, b| {
        if dirs_first && a.is_dir != b.is_dir {
            return if a.is_dir {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }

        let ord = match key {
            SortKey::Name => natural_cmp(&a.name, &b.name),
            SortKey::Size => a.size.cmp(&b.size).then_with(|| natural_cmp(&a.name, &b.name)),
            SortKey::Modified => a
                .modified
                .cmp(&b.modified)
                .then_with(|| natural_cmp(&a.name, &b.name)),
            SortKey::Kind => a
                .kind
                .cmp(&b.kind)
                .then_with(|| natural_cmp(&a.name, &b.name)),
        };

        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });
}

/// Сравнение с учётом чисел: `файл2` идёт раньше `файл10`.
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ac), Some(bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let an = take_number(&mut ai);
                    let bn = take_number(&mut bi);
                    match an.cmp(&bn) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }

                let al = ac.to_lowercase().next().unwrap_or(ac);
                let bl = bc.to_lowercase().next().unwrap_or(bc);
                match al.cmp(&bl) {
                    Ordering::Equal => {
                        let _ = ai.next();
                        let _ = bi.next();
                    }
                    other => return other,
                }
            }
        }
    }
}

fn take_number(iter: &mut std::iter::Peekable<std::str::Chars>) -> u128 {
    let mut value: u128 = 0;
    while let Some(c) = iter.peek().copied() {
        if !c.is_ascii_digit() {
            break;
        }
        // Очень длинные числа просто насыщаем — для сортировки этого хватает.
        value = value.saturating_mul(10).saturating_add((c as u8 - b'0') as u128);
        let _ = iter.next();
    }
    value
}

/// «1,4 ГБ», «376 КБ», «12 Б».
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["Б", "КБ", "МБ", "ГБ", "ТБ"];
    if bytes < 1024 {
        return format!("{bytes} Б");
    }

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    let text = if value < 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    };

    format!("{} {}", text.replace('.', ","), UNITS[unit])
}

/// Локальное время в виде `2026-08-12 16:24`.
pub fn format_time(time: SystemTime) -> String {
    let Ok(elapsed) = time.duration_since(UNIX_EPOCH) else {
        return "—".into();
    };

    let secs = elapsed.as_secs() as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let ok = unsafe { !libc::localtime_r(&secs, &mut tm).is_null() };
    if !ok {
        return "—".into();
    }

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min
    )
}

/// Длительность в «2 мин 5 с» для оценки времени копирования.
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs} с")
    } else if secs < 3600 {
        format!("{} мин {} с", secs / 60, secs % 60)
    } else {
        format!("{} ч {} мин", secs / 3600, (secs % 3600) / 60)
    }
}
