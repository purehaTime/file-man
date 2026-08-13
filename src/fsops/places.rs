//! Левая панель: места быстрого доступа и список смонтированных дисков.

use std::path::{Path, PathBuf};

use crate::i18n::{Lang, S};
use crate::ipc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceKind {
    Home,
    Desktop,
    Documents,
    Downloads,
    Music,
    Pictures,
    Videos,
    Folder,
    Trash,
    Root,
    Drive,
    Removable,
}

#[derive(Debug, Clone)]
pub struct Place {
    pub label: String,
    pub path: PathBuf,
    pub kind: PlaceKind,
    /// (свободно, всего) — только для дисков.
    pub usage: Option<(u64, u64)>,
}

pub fn home() -> PathBuf {
    if let Some(dir) = std::env::var_os("HOME") {
        return PathBuf::from(dir);
    }
    PathBuf::from("/")
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir);
    }
    home().join(".config")
}

pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir);
    }
    home().join(".local/share")
}

/// Места быстрого доступа: стандартные каталоги пользователя плюс закладки
/// из `~/.config/gtk-3.0/bookmarks` (общий для многих менеджеров файл).
pub fn quick_access(lang: Lang, extra: &[PathBuf]) -> Vec<Place> {
    let home = home();
    let dirs = user_dirs();

    let mut places = vec![Place {
        label: lang.s(S::PlaceHome).into(),
        path: home.clone(),
        kind: PlaceKind::Home,
        usage: None,
    }];

    let standard: [(&str, PlaceKind, &str, S); 6] = [
        ("XDG_DESKTOP_DIR", PlaceKind::Desktop, "Desktop", S::PlaceDesktop),
        ("XDG_DOWNLOAD_DIR", PlaceKind::Downloads, "Downloads", S::PlaceDownloads),
        ("XDG_DOCUMENTS_DIR", PlaceKind::Documents, "Documents", S::PlaceDocuments),
        ("XDG_PICTURES_DIR", PlaceKind::Pictures, "Pictures", S::PlacePictures),
        ("XDG_MUSIC_DIR", PlaceKind::Music, "Music", S::PlaceMusic),
        ("XDG_VIDEOS_DIR", PlaceKind::Videos, "Videos", S::PlaceVideos),
    ];

    for (key, kind, fallback, label) in standard {
        let path = dirs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| home.join(fallback));

        if path.is_dir() && path != home {
            places.push(Place {
                label: lang.s(label).into(),
                path,
                kind,
                usage: None,
            });
        }
    }

    for path in bookmarks().into_iter().chain(extra.iter().cloned()) {
        if !path.is_dir() || places.iter().any(|p| p.path == path) {
            continue;
        }
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        places.push(Place {
            label,
            path,
            kind: PlaceKind::Folder,
            usage: None,
        });
    }

    let trash = data_dir().join("Trash/files");
    if trash.is_dir() {
        places.push(Place {
            label: lang.s(S::PlaceTrash).into(),
            path: trash,
            kind: PlaceKind::Trash,
            usage: None,
        });
    }

    places
}

/// Разбор `~/.config/user-dirs.dirs`.
fn user_dirs() -> Vec<(String, PathBuf)> {
    let path = config_dir().join("user-dirs.dirs");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let home = home();

    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim().trim_matches('"');
            let value = if let Some(rest) = value.strip_prefix("$HOME/") {
                home.join(rest)
            } else if value == "$HOME" {
                home.clone()
            } else {
                PathBuf::from(value)
            };
            Some((key.trim().to_string(), value))
        })
        .collect()
}

fn bookmarks() -> Vec<PathBuf> {
    let path = config_dir().join("gtk-3.0/bookmarks");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    text.lines()
        .filter_map(|line| {
            let uri = line.split_whitespace().next()?;
            let rest = uri.strip_prefix("file://")?;
            Some(PathBuf::from(percent_decode(rest)))
        })
        .collect()
}

pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// Смонтированные диски из `/proc/mounts`.
pub fn drives(lang: Lang) -> Vec<Place> {
    const SKIP_FS: &[&str] = &[
        "proc", "sysfs", "devtmpfs", "devpts", "tmpfs", "cgroup", "cgroup2", "mqueue",
        "hugetlbfs", "debugfs", "tracefs", "configfs", "securityfs", "pstore", "bpf", "autofs",
        "fusectl", "efivarfs", "ramfs", "binfmt_misc", "nsfs", "overlay", "squashfs", "selinuxfs",
        "rpc_pipefs", "fuse.gvfsd-fuse", "fuse.portal", "nfsd", "sunrpc", "gvfsd-fuse",
    ];

    let Ok(text) = std::fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };

    let media_roots = ["/run/media", "/media", "/mnt", "/run/mount"];
    let mut out: Vec<Place> = Vec::new();

    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let device = unescape_octal(fields.next().unwrap_or_default());
        let mount = unescape_octal(fields.next().unwrap_or_default());
        let fstype = fields.next().unwrap_or_default();

        if mount.is_empty() || SKIP_FS.contains(&fstype) {
            continue;
        }

        let is_block = device.starts_with("/dev/");
        let in_media = media_roots.iter().any(|root| mount.starts_with(root));
        let is_network = matches!(fstype, "nfs" | "nfs4" | "cifs" | "smb3" | "sshfs" | "fuse.sshfs");

        if !is_block && !in_media && !is_network {
            continue;
        }

        let mount_path = PathBuf::from(&mount);
        if out.iter().any(|p| p.path == mount_path) {
            continue;
        }

        let removable = is_block && is_removable(&device) || in_media;

        let label = if mount == "/" {
            lang.s(S::PlaceRoot).to_string()
        } else {
            mount_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| mount.clone())
        };

        out.push(Place {
            label,
            kind: if mount == "/" {
                PlaceKind::Root
            } else if removable {
                PlaceKind::Removable
            } else {
                PlaceKind::Drive
            },
            usage: ipc::disk_usage(&mount_path),
            path: mount_path,
        });
    }

    // Корень всегда первым, дальше — по точке монтирования.
    out.sort_by(|a, b| {
        let rank = |p: &Place| if p.kind == PlaceKind::Root { 0 } else { 1 };
        rank(a).cmp(&rank(b)).then_with(|| a.path.cmp(&b.path))
    });

    out
}

/// `/proc/mounts` экранирует пробелы и табуляции восьмеричными кодами.
fn unescape_octal(input: &str) -> String {
    if !input.contains('\\') {
        return input.to_string();
    }

    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            let oct = std::str::from_utf8(&bytes[i + 1..i + 4]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(oct, 8) {
                out.push(byte);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// Съёмный ли носитель: `/sys/block/<диск>/removable`.
fn is_removable(device: &str) -> bool {
    let Some(name) = device.strip_prefix("/dev/") else {
        return false;
    };

    let disk = base_disk(name);
    let flag = Path::new("/sys/block").join(&disk).join("removable");

    std::fs::read_to_string(flag)
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

/// `sda1` → `sda`, `nvme0n1p3` → `nvme0n1`, `mmcblk0p1` → `mmcblk0`.
fn base_disk(name: &str) -> String {
    if let Some(pos) = name.rfind('p') {
        let (head, tail) = name.split_at(pos);
        let digits_tail = tail[1..].chars().all(|c| c.is_ascii_digit()) && tail.len() > 1;
        let digit_before = head.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false);
        if digits_tail && digit_before {
            return head.to_string();
        }
    }

    name.trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}
