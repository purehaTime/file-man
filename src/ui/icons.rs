//! Векторные иконки, встроенные в бинарник.
//!
//! Ни GTK, ни Qt, ни тем оформления снаружи: каждая иконка — небольшой SVG,
//! который iced растеризует и перекрашивает под цвет темы (фильтр заменяет
//! RGB, сохраняя альфу, поэтому рисунки одноцветные).

use iced::widget::svg;
use iced::widget::svg::Handle;
use iced::{Color, Element, Length};

use crate::fsops::{Kind, PlaceKind};

macro_rules! icon {
    ($body:expr) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" "#,
            r#"stroke="black" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</svg>"
        )
    };
}

// ------------------------------------------------------------ типы файлов

pub const FOLDER: &str = icon!(
    r#"<path d="M3 6.6A1.6 1.6 0 0 1 4.6 5h4l2.2 2.3h8.6A1.6 1.6 0 0 1 21 8.9v9.5A1.6 1.6 0 0 1 19.4 20H4.6A1.6 1.6 0 0 1 3 18.4z"/>"#
);

pub const FILE: &str = icon!(
    r#"<path d="M14 3.2H7.2A1.7 1.7 0 0 0 5.5 4.9v14.2a1.7 1.7 0 0 0 1.7 1.7h9.6a1.7 1.7 0 0 0 1.7-1.7V7.6z"/><path d="M14 3.2v4.4h4.5"/>"#
);

pub const FILE_IMAGE: &str = icon!(
    r#"<rect x="3.2" y="5" width="17.6" height="14" rx="2"/><circle cx="8.6" cy="10.1" r="1.5"/><path d="M4 17.4l4.6-4.5 2.9 2.9 3.4-3.9 5.1 5.5"/>"#
);

pub const FILE_VIDEO: &str = icon!(
    r#"<rect x="3.2" y="5" width="17.6" height="14" rx="2"/><path d="M3.2 9h17.6M3.2 15h17.6M8 5v14M16 5v14"/>"#
);

pub const FILE_AUDIO: &str = icon!(
    r#"<path d="M9 17.5V6.2l10-2v11.1"/><circle cx="6.6" cy="17.6" r="2.4"/><circle cx="16.6" cy="15.4" r="2.4"/>"#
);

pub const FILE_ARCHIVE: &str = icon!(
    r#"<rect x="3.2" y="4.2" width="17.6" height="4.6" rx="1.4"/><path d="M5.2 8.8v10.2A1.7 1.7 0 0 0 6.9 20.7h10.2a1.7 1.7 0 0 0 1.7-1.7V8.8"/><path d="M10.2 12.6h3.6"/>"#
);

pub const FILE_CODE: &str = icon!(
    r#"<path d="M9.4 8.4 4.6 12l4.8 3.6"/><path d="M14.6 8.4 19.4 12l-4.8 3.6"/>"#
);

pub const FILE_TEXT: &str = icon!(
    r#"<path d="M14 3.2H7.2A1.7 1.7 0 0 0 5.5 4.9v14.2a1.7 1.7 0 0 0 1.7 1.7h9.6a1.7 1.7 0 0 0 1.7-1.7V7.6z"/><path d="M14 3.2v4.4h4.5"/><path d="M8.6 12.4h6.8M8.6 15.8h4.6"/>"#
);

pub const FILE_PDF: &str = icon!(
    r#"<path d="M14 3.2H7.2A1.7 1.7 0 0 0 5.5 4.9v14.2a1.7 1.7 0 0 0 1.7 1.7h9.6a1.7 1.7 0 0 0 1.7-1.7V7.6z"/><path d="M14 3.2v4.4h4.5"/><path d="M9 18v-5.4h2a1.7 1.7 0 0 1 0 3.4H9"/>"#
);

pub const FILE_DOC: &str = icon!(
    r#"<path d="M14 3.2H7.2A1.7 1.7 0 0 0 5.5 4.9v14.2a1.7 1.7 0 0 0 1.7 1.7h9.6a1.7 1.7 0 0 0 1.7-1.7V7.6z"/><path d="M14 3.2v4.4h4.5"/><path d="M8.6 12.4h6.8M8.6 15.4h6.8M8.6 18.2h3.8"/>"#
);

pub const FILE_EXEC: &str = icon!(
    r#"<rect x="3.2" y="4.6" width="17.6" height="14.8" rx="2"/><path d="M7.4 10.2 10 12.4l-2.6 2.2M12.8 15.2h4"/>"#
);

// ------------------------------------------------------------ левая панель

pub const HOME: &str = icon!(
    r#"<path d="M4 10.4 12 4l8 6.4V19a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 19z"/><path d="M9.6 20.5v-6.2h4.8v6.2"/>"#
);

pub const DESKTOP: &str = icon!(
    r#"<rect x="3" y="4.4" width="18" height="12.4" rx="1.8"/><path d="M8.4 20.4h7.2M12 16.8v3.6"/>"#
);

pub const DOWNLOADS: &str = icon!(
    r#"<path d="M12 3.6v10.2"/><path d="M8.2 10.2 12 14l3.8-3.8"/><path d="M4.4 16.6v2.2a1.6 1.6 0 0 0 1.6 1.6h12a1.6 1.6 0 0 0 1.6-1.6v-2.2"/>"#
);

pub const MUSIC: &str = FILE_AUDIO;
pub const PICTURES: &str = FILE_IMAGE;
pub const VIDEOS: &str = FILE_VIDEO;
pub const DOCUMENTS: &str = FILE_DOC;

pub const TRASH: &str = icon!(
    r#"<path d="M4 6.6h16"/><path d="M9.4 6.6V4.9A1.4 1.4 0 0 1 10.8 3.5h2.4a1.4 1.4 0 0 1 1.4 1.4v1.7"/><path d="M6.2 6.6 7 19.2a1.6 1.6 0 0 0 1.6 1.5h6.8a1.6 1.6 0 0 0 1.6-1.5l.8-12.6"/><path d="M10.4 10.4v6.4M13.6 10.4v6.4"/>"#
);

pub const DRIVE: &str = icon!(
    r#"<rect x="3" y="5" width="18" height="6" rx="1.8"/><rect x="3" y="13" width="18" height="6" rx="1.8"/><path d="M6.8 8h.01M6.8 16h.01"/>"#
);

pub const REMOVABLE: &str = icon!(
    r#"<path d="M8 20.6h8a1.6 1.6 0 0 0 1.6-1.6V9.4L14 3.4H8A1.6 1.6 0 0 0 6.4 5v14A1.6 1.6 0 0 0 8 20.6z"/><path d="M9.6 7.2h2.8M9.6 10.4h2.8"/>"#
);

pub const ROOT: &str = icon!(
    r#"<ellipse cx="12" cy="6.2" rx="7.6" ry="2.8"/><path d="M4.4 6.2v11.6c0 1.6 3.4 2.8 7.6 2.8s7.6-1.2 7.6-2.8V6.2"/><path d="M4.4 12c0 1.6 3.4 2.8 7.6 2.8s7.6-1.2 7.6-2.8"/>"#
);

// ------------------------------------------------------------ управление

pub const ARROW_LEFT: &str = icon!(r#"<path d="M15 5.4 8.2 12l6.8 6.6"/>"#);
pub const ARROW_RIGHT: &str = icon!(r#"<path d="M9 5.4 15.8 12 9 18.6"/>"#);
pub const ARROW_UP: &str = icon!(r#"<path d="M12 19.4V5.2"/><path d="M5.6 11.4 12 5l6.4 6.4"/>"#);
pub const CHEVRON_RIGHT: &str = icon!(r#"<path d="M10 6.6 15.4 12 10 17.4"/>"#);

pub const REFRESH: &str = icon!(
    r#"<path d="M20 12a8 8 0 1 1-2.4-5.7"/><path d="M20.4 4.4v4.4H16"/>"#
);

pub const SEARCH: &str = icon!(r#"<circle cx="10.8" cy="10.8" r="6.4"/><path d="M15.6 15.6 20.4 20.4"/>"#);

pub const EYE: &str = icon!(
    r#"<path d="M2.6 12S6.4 5.4 12 5.4 21.4 12 21.4 12 17.6 18.6 12 18.6 2.6 12 2.6 12z"/><circle cx="12" cy="12" r="2.8"/>"#
);

pub const EYE_OFF: &str = icon!(
    r#"<path d="M9.6 6a8.6 8.6 0 0 1 2.4-.3c5.6 0 9.4 6.3 9.4 6.3a17 17 0 0 1-3 3.7M6.6 7.6A17 17 0 0 0 2.6 12s3.8 6.3 9.4 6.3a8.6 8.6 0 0 0 3.6-.8"/><path d="M4 4 20 20"/>"#
);

pub const PALETTE: &str = icon!(
    r#"<path d="M12 3.4a8.6 8.6 0 0 0 0 17.2c1.3 0 2-.9 2-1.8 0-1.5-1.4-1.6-1.4-2.9 0-.9.7-1.6 1.7-1.6h1.9a4.4 4.4 0 0 0 4.4-4.4c0-3.6-3.7-6.5-8.6-6.5z"/><circle cx="8" cy="10" r="1.1"/><circle cx="12" cy="7.6" r="1.1"/><circle cx="16" cy="10" r="1.1"/>"#
);

pub const VIEW_DETAILS: &str = icon!(
    r#"<path d="M4 6.6h3M10 6.6h10M4 12h3M10 12h10M4 17.4h3M10 17.4h10"/>"#
);

pub const VIEW_COMPACT: &str = icon!(
    r#"<path d="M4 5.4v13.2M12 5.4v13.2M20 5.4v13.2"/><path d="M5.6 8h4.8M5.6 12h4.8M13.6 8h4.8M13.6 12h4.8"/>"#
);

pub const VIEW_ICONS: &str = icon!(
    r#"<rect x="4" y="4" width="6.6" height="6.6" rx="1.4"/><rect x="13.4" y="4" width="6.6" height="6.6" rx="1.4"/><rect x="4" y="13.4" width="6.6" height="6.6" rx="1.4"/><rect x="13.4" y="13.4" width="6.6" height="6.6" rx="1.4"/>"#
);

pub const FOLDER_PLUS: &str = icon!(
    r#"<path d="M3 6.6A1.6 1.6 0 0 1 4.6 5h4l2.2 2.3h8.6A1.6 1.6 0 0 1 21 8.9v9.5A1.6 1.6 0 0 1 19.4 20H4.6A1.6 1.6 0 0 1 3 18.4z"/><path d="M12 11.4v5.2M9.4 14h5.2"/>"#
);

pub const COPY: &str = icon!(
    r#"<rect x="8.6" y="8.6" width="11.4" height="11.4" rx="1.8"/><path d="M15.4 5.4H5.8A1.8 1.8 0 0 0 4 7.2v9.6"/>"#
);

pub const CUT: &str = icon!(
    r#"<circle cx="6.4" cy="17.6" r="2.6"/><circle cx="17.6" cy="17.6" r="2.6"/><path d="M8.4 15.6 18 4.4M15.6 15.6 6 4.4"/>"#
);

pub const PASTE: &str = icon!(
    r#"<path d="M9 4.6H7A1.8 1.8 0 0 0 5.2 6.4v12.4A1.8 1.8 0 0 0 7 20.6h10a1.8 1.8 0 0 0 1.8-1.8V6.4A1.8 1.8 0 0 0 17 4.6h-2"/><rect x="9" y="2.8" width="6" height="3.6" rx="1.2"/>"#
);

pub const RENAME: &str = icon!(
    r#"<path d="M14.6 4.8 19.2 9.4M4 20l.9-3.9L15.4 5.6a1.7 1.7 0 0 1 2.4 0l.6.6a1.7 1.7 0 0 1 0 2.4L7.9 19.1z"/>"#
);

pub const INFO: &str = icon!(
    r#"<circle cx="12" cy="12" r="8.6"/><path d="M12 11.2v5M12 8.2h.01"/>"#
);

pub const ALERT: &str = icon!(
    r#"<path d="M12 4.2 21 19.4H3z"/><path d="M12 10v4M12 16.6h.01"/>"#
);

pub const CLOSE: &str = icon!(r#"<path d="M5.6 5.6 18.4 18.4M18.4 5.6 5.6 18.4"/>"#);
pub const PAUSE: &str = icon!(r#"<path d="M9.4 5.4v13.2M14.6 5.4v13.2"/>"#);
pub const PLAY: &str = icon!(r#"<path d="M7.6 4.8 19 12 7.6 19.2z"/>"#);
pub const CHECK: &str = icon!(r#"<path d="M4.6 12.8 9.4 17.6 19.4 6.4"/>"#);
pub const SORT_ASC: &str = icon!(r#"<path d="M12 18.6V5.4"/><path d="M7.4 10 12 5.4 16.6 10"/>"#);
pub const SORT_DESC: &str = icon!(r#"<path d="M12 5.4v13.2"/><path d="M7.4 14 12 18.6 16.6 14"/>"#);
pub const LINK: &str = icon!(
    r#"<path d="M10.4 13.6a4 4 0 0 0 5.7 0l2.8-2.8a4 4 0 0 0-5.7-5.7l-1.4 1.4"/><path d="M13.6 10.4a4 4 0 0 0-5.7 0l-2.8 2.8a4 4 0 0 0 5.7 5.7l1.4-1.4"/>"#
);

// ------------------------------------------------------------ построение

pub fn for_kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Folder => FOLDER,
        Kind::Image => FILE_IMAGE,
        Kind::Video => FILE_VIDEO,
        Kind::Audio => FILE_AUDIO,
        Kind::Archive => FILE_ARCHIVE,
        Kind::Code => FILE_CODE,
        Kind::Document => FILE_DOC,
        Kind::Pdf => FILE_PDF,
        Kind::Text => FILE_TEXT,
        Kind::Executable => FILE_EXEC,
        Kind::Unknown => FILE,
    }
}

pub fn for_place(kind: PlaceKind) -> &'static str {
    match kind {
        PlaceKind::Home => HOME,
        PlaceKind::Desktop => DESKTOP,
        PlaceKind::Documents => DOCUMENTS,
        PlaceKind::Downloads => DOWNLOADS,
        PlaceKind::Music => MUSIC,
        PlaceKind::Pictures => PICTURES,
        PlaceKind::Videos => VIDEOS,
        PlaceKind::Folder => FOLDER,
        PlaceKind::Trash => TRASH,
        PlaceKind::Root => ROOT,
        PlaceKind::Drive => DRIVE,
        PlaceKind::Removable => REMOVABLE,
    }
}

/// Иконка цветом текста темы.
pub fn view<'a, Message: 'a>(source: &'static str, size: u16) -> Element<'a, Message> {
    build(source, size, None)
}

/// Иконка заданным цветом.
pub fn colored<'a, Message: 'a>(
    source: &'static str,
    size: u16,
    color: Color,
) -> Element<'a, Message> {
    build(source, size, Some(color))
}

fn build<'a, Message: 'a>(
    source: &'static str,
    size: u16,
    color: Option<Color>,
) -> Element<'a, Message> {
    svg(Handle::from_memory(source.as_bytes()))
        .width(Length::Fixed(size as f32))
        .height(Length::Fixed(size as f32))
        .style(move |theme: &iced::Theme, _status| svg::Style {
            color: Some(color.unwrap_or_else(|| theme.extended_palette().background.base.text)),
        })
        .into()
}
