//! Работа с файловой системой: чтение каталогов, места быстрого доступа,
//! диски и операции над файлами.

pub mod entry;
pub mod ops;
pub mod places;

pub use entry::{Entry, Kind, SortKey};
pub use places::{Place, PlaceKind};
