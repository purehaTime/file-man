//! Нижняя строка: сводка по каталогу и прогресс фоновых задач.

use iced::widget::{button, container, progress_bar, row, space, text, tooltip};
use iced::{Center, Element, Fill, Length};

use super::{icons, style, App, Message};
use crate::fsops::entry::{format_duration, format_size};
use crate::ipc::{Job, JobState};

pub fn view(app: &App) -> Element<'_, Message> {
    let bar = row![summary(app), space().width(Fill), jobs(app)]
        .spacing(12)
        .align_y(Center)
        .padding([5, 10]);

    iced::widget::column![
        super::toolbar::separator(),
        container(bar).width(Fill).style(style::chrome),
    ]
    .into()
}

/// Что сейчас в каталоге и сколько выделено.
fn summary(app: &App) -> Element<'_, Message> {
    let total = app.filtered.len();
    let dirs = app
        .visible_entries()
        .filter(|(_, entry)| entry.is_dir)
        .count();
    let files = total - dirs;

    let mut parts = Vec::new();
    if dirs > 0 {
        parts.push(format!("{dirs} {}", plural(dirs, "папка", "папки", "папок")));
    }
    if files > 0 {
        parts.push(format!(
            "{files} {}",
            plural(files, "файл", "файла", "файлов")
        ));
    }
    if parts.is_empty() {
        parts.push("пусто".into());
    }

    let mut line = row![text(parts.join(", ")).size(12).style(style::muted)]
        .spacing(10)
        .align_y(Center);

    let selected: Vec<_> = app
        .visible_entries()
        .filter(|(_, entry)| app.selection.contains(&entry.path))
        .map(|(_, entry)| entry)
        .collect();

    if !selected.is_empty() {
        let bytes: u64 = selected.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
        let label = if bytes > 0 {
            format!(
                "выбрано {} ({})",
                selected.len(),
                format_size(bytes)
            )
        } else {
            format!("выбрано {}", selected.len())
        };
        line = line.push(text(label).size(12).style(style::accent_text));
    }

    if let Some((free, total_bytes)) = app.usage {
        if total_bytes > 0 {
            line = line.push(
                text(format!("· свободно {}", format_size(free)))
                    .size(12)
                    .style(style::muted),
            );
        }
    }

    if app.loading {
        line = line.push(text("· обновление…").size(12).style(style::muted));
    }

    line.into()
}

/// Прогресс копирования. Задачи живут в демоне, поэтому строка одинаково
/// корректна и сразу после запуска нового окна.
fn jobs(app: &App) -> Element<'_, Message> {
    let active: Vec<&Job> = app.active_jobs().collect();
    let finished = app.jobs.len() - active.len();

    if let Some(job) = active.first() {
        let mut line = row![
            text(job.title())
                .size(12)
                .wrapping(text::Wrapping::None),
            progress_bar(0.0..=1.0, job.fraction())
                .length(Length::Fixed(150.0))
                .girth(Length::Fixed(6.0))
                .style(|theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    progress_bar::Style {
                        background: iced::Background::Color(palette.background.strong.color),
                        bar: iced::Background::Color(palette.primary.base.color),
                        border: iced::Border {
                            radius: 3.0.into(),
                            ..iced::Border::default()
                        },
                    }
                }),
            text(details(job)).size(12).style(style::muted),
        ]
        .spacing(10)
        .align_y(Center);

        if active.len() > 1 {
            line = line.push(
                button(text(format!("+{}", active.len() - 1)).size(12))
                    .padding([2, 6])
                    .style(style::crumb)
                    .on_press(Message::ShowJobs),
            );
        }

        let toggle = if job.state == JobState::Paused {
            small(icons::PLAY, "Продолжить", Message::ResumeJob(job.id))
        } else {
            small(icons::PAUSE, "Пауза", Message::PauseJob(job.id))
        };

        line = line
            .push(toggle)
            .push(small(icons::CLOSE, "Отменить", Message::CancelJob(job.id)));

        return line.align_y(Center).into();
    }

    if finished > 0 {
        let last = app.jobs.last();
        let (icon, label) = match last.map(|job| &job.state) {
            Some(JobState::Failed(_)) => (icons::ALERT, "Задача завершилась с ошибкой"),
            Some(JobState::Cancelled) => (icons::INFO, "Задача отменена"),
            _ => (icons::CHECK, "Копирование завершено"),
        };

        return row![
            button(
                row![icons::view(icon, 14), text(label).size(12)]
                    .spacing(6)
                    .align_y(Center)
            )
            .padding([2, 6])
            .style(style::crumb)
            .on_press(Message::ShowJobs),
            small(icons::CLOSE, "Скрыть", Message::DismissFinished),
        ]
        .spacing(6)
        .align_y(Center)
        .into();
    }

    if !app.daemon_online {
        return row![
            icons::view(icons::ALERT, 13),
            text("служба копирования недоступна")
                .size(12)
                .style(style::muted),
        ]
        .spacing(6)
        .align_y(Center)
        .into();
    }

    space().width(Length::Fixed(0.0)).into()
}

fn details(job: &Job) -> String {
    match &job.state {
        JobState::Scanning => "подсчёт объёма…".into(),
        JobState::Paused => "пауза".into(),
        JobState::Running => {
            let mut parts = vec![format!("{:.0}%", job.fraction() * 100.0)];
            if job.speed > 0 {
                parts.push(format!("{}/с", format_size(job.speed)));
            }
            if let Some(eta) = job.eta {
                parts.push(format!("осталось {}", format_duration(eta)));
            }
            parts.join(" · ")
        }
        other => other.label().to_string(),
    }
}

fn small<'a>(icon: &'static str, hint: &'a str, message: Message) -> Element<'a, Message> {
    tooltip(
        button(icons::view(icon, 13))
            .padding(4)
            .style(style::tool)
            .on_press(message),
        container(text(hint).size(12))
            .padding([4, 8])
            .style(style::popup),
        tooltip::Position::Top,
    )
    .gap(4)
    .into()
}

fn plural<'a>(count: usize, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
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
