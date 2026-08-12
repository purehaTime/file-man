//! Контекстное меню и модальные окна.

use iced::widget::{
    button, column, container, mouse_area, opaque, progress_bar, row, scrollable, space, text,
    text_input,
};
use iced::{Center, Element, Fill, Length, Size};

use super::{clamp_menu, icons, pinned, style, App, Context, Message, Modal, DIALOG_ID};
use crate::fsops::entry::{format_size, format_time};
use crate::fsops::ops;
use crate::ipc::{Job, JobState};

const MENU_WIDTH: f32 = 250.0;
const MENU_ITEM: f32 = 30.0;

// ------------------------------------------------------------ меню по ПКМ

pub fn context_menu<'a>(app: &'a App, context: &'a Context) -> Element<'a, Message> {
    let on_entry = context.target.is_some();
    let can_paste = app.clipboard.is_some() && ops::is_writable(&app.dir);
    let selected = app.selection.len();

    let mut items: Vec<Element<'a, Message>> = Vec::new();
    let mut height = 12.0;

    if on_entry {
        items.push(entry_item(icons::PLAY, "Открыть", Some(Message::OpenSelected)));
        items.push(divider());
        items.push(entry_item(
            icons::COPY,
            "Копировать",
            Some(Message::CopySelection),
        ));
        items.push(entry_item(icons::CUT, "Вырезать", Some(Message::CutSelection)));
        height += MENU_ITEM * 3.0 + 9.0;
    }

    items.push(entry_item(
        icons::PASTE,
        "Вставить",
        can_paste.then_some(Message::Paste),
    ));
    height += MENU_ITEM;

    if on_entry {
        items.push(divider());
        items.push(entry_item(
            icons::RENAME,
            "Переименовать",
            (selected == 1).then_some(Message::RequestRename),
        ));
        items.push(entry_item(
            icons::TRASH,
            "В корзину",
            Some(Message::RequestDelete { permanent: false }),
        ));
        items.push(entry_item(
            icons::CLOSE,
            "Удалить навсегда",
            Some(Message::RequestDelete { permanent: true }),
        ));
        height += MENU_ITEM * 3.0 + 9.0;
    }

    items.push(divider());
    items.push(entry_item(
        icons::FOLDER_PLUS,
        "Создать папку",
        Some(Message::RequestNewFolder),
    ));
    items.push(entry_item(
        icons::FILE,
        "Создать файл",
        Some(Message::RequestNewFile),
    ));
    height += MENU_ITEM * 2.0 + 9.0;

    if on_entry {
        items.push(divider());
        items.push(entry_item(
            icons::INFO,
            "Свойства",
            (selected == 1).then_some(Message::RequestProperties),
        ));
        height += MENU_ITEM + 9.0;
    }

    let menu = container(column(items).spacing(1))
        .width(Length::Fixed(MENU_WIDTH))
        .padding(6)
        .style(style::popup);

    let position = clamp_menu(app, context.position, Size::new(MENU_WIDTH, height));

    iced::widget::stack![
        mouse_area(container(space()).width(Fill).height(Fill))
            .on_press(Message::ContextClose)
            .on_right_press(Message::ContextClose),
        pinned(menu.into(), position),
    ]
    .into()
}

fn entry_item<'a>(
    icon: &'static str,
    label: &'a str,
    message: Option<Message>,
) -> Element<'a, Message> {
    button(
        row![icons::view(icon, 15), text(label).size(13)]
            .spacing(10)
            .align_y(Center),
    )
    .width(Fill)
    .padding([6, 8])
    .style(style::menu_item)
    .on_press_maybe(message)
    .into()
}

fn divider<'a>() -> Element<'a, Message> {
    container(space())
        .width(Fill)
        .height(Length::Fixed(1.0))
        .padding([4, 0])
        .style(|theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(
                theme.extended_palette().background.strong.color,
            )),
            ..container::Style::default()
        })
        .into()
}

// ------------------------------------------------------------ модальные окна

pub fn modal<'a>(app: &'a App, modal: &'a Modal) -> Element<'a, Message> {
    let body = match modal {
        Modal::Rename { value, .. } => prompt("Переименовать", "Новое имя", value),
        Modal::NewFolder { value } => prompt("Новая папка", "Имя папки", value),
        Modal::NewFile { value } => prompt("Новый файл", "Имя файла", value),
        Modal::ConfirmDelete { paths, permanent } => confirm_delete(paths, *permanent),
        Modal::Properties { entry, contents } => properties(entry, *contents),
        Modal::Jobs => jobs_list(app),
        Modal::Error { title, message } => error_box(title, message),
    };

    opaque(body)
}

fn frame<'a>(title: &'a str, content: Element<'a, Message>) -> Element<'a, Message> {
    container(
        column![text(title).size(16), content]
            .spacing(14)
            .width(Fill),
    )
    .width(Length::Fixed(460.0))
    .padding(18)
    .style(style::popup)
    .into()
}

fn prompt<'a>(title: &'a str, placeholder: &'a str, value: &'a str) -> Element<'a, Message> {
    let content = column![
        text_input(placeholder, value)
            .id(DIALOG_ID)
            .on_input(Message::ModalInput)
            .on_submit(Message::ModalSubmit)
            .padding([8, 10])
            .size(14)
            .style(style::input),
        buttons("Отмена", "Готово", style::primary, Message::ModalSubmit),
    ]
    .spacing(16);

    frame(title, content.into())
}

fn confirm_delete<'a>(paths: &'a [std::path::PathBuf], permanent: bool) -> Element<'a, Message> {
    let title = if permanent {
        "Удалить безвозвратно?"
    } else {
        "Переместить в корзину?"
    };

    let mut list = column![].spacing(3);
    for path in paths.iter().take(8) {
        list = list.push(
            text(
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
            )
            .size(13)
            .style(style::muted),
        );
    }
    if paths.len() > 8 {
        list = list.push(
            text(format!("…и ещё {}", paths.len() - 8))
                .size(13)
                .style(style::muted),
        );
    }

    let warning = if permanent {
        "Восстановить эти объекты будет невозможно."
    } else {
        "Объекты можно будет вернуть из корзины."
    };

    let content = column![
        text(warning).size(13),
        list,
        buttons(
            "Отмена",
            if permanent { "Удалить" } else { "В корзину" },
            if permanent { style::danger } else { style::primary },
            Message::ModalSubmit,
        ),
    ]
    .spacing(14);

    frame(title, content.into())
}

fn properties<'a>(
    entry: &'a crate::fsops::Entry,
    contents: Option<usize>,
) -> Element<'a, Message> {
    let mut rows = column![
        field("Имя", entry.name.clone()),
        field("Путь", entry.path.display().to_string()),
        field("Тип", entry.kind.label().to_string()),
    ]
    .spacing(8);

    if entry.is_dir {
        if let Some(count) = contents {
            rows = rows.push(field("Содержимое", format!("{count} объектов")));
        }
    } else {
        rows = rows.push(field(
            "Размер",
            format!("{} ({} Б)", format_size(entry.size), entry.size),
        ));
    }

    if let Some(modified) = entry.modified {
        rows = rows.push(field("Изменён", format_time(modified)));
    }
    if entry.hidden {
        rows = rows.push(field("Скрытый", "да".into()));
    }
    if entry.is_symlink {
        let target = std::fs::read_link(&entry.path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "—".into());
        rows = rows.push(field("Ссылка на", target));
    }

    let content = column![
        rows,
        row![
            space().width(Fill),
            button(text("Закрыть").size(13))
                .padding([8, 16])
                .style(style::neutral)
                .on_press(Message::ModalClose),
        ],
    ]
    .spacing(16);

    frame("Свойства", content.into())
}

fn field<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    row![
        text(label).size(12).width(Length::Fixed(110.0)).style(style::muted),
        text(value).size(13).width(Fill),
    ]
    .spacing(10)
    .into()
}

fn jobs_list(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(10);

    if app.jobs.is_empty() {
        list = list.push(text("Фоновых задач нет").size(13).style(style::muted));
    }

    for job in &app.jobs {
        list = list.push(job_card(job));
    }

    let content = column![
        scrollable(list).height(Length::Fixed(320.0)),
        row![
            button(text("Убрать завершённые").size(13))
                .padding([8, 14])
                .style(style::neutral)
                .on_press(Message::DismissFinished),
            space().width(Fill),
            button(text("Закрыть").size(13))
                .padding([8, 16])
                .style(style::primary)
                .on_press(Message::ModalClose),
        ]
        .spacing(10)
        .align_y(Center),
    ]
    .spacing(16);

    frame("Фоновые задачи", content.into())
}

fn job_card(job: &Job) -> Element<'_, Message> {
    let mut header = row![
        text(job.title()).size(13).width(Fill),
        text(job.state.label()).size(12).style(style::muted),
    ]
    .spacing(10)
    .align_y(Center);

    if job.state.is_active() {
        header = header.push(
            button(icons::view(icons::CLOSE, 13))
                .padding(4)
                .style(style::tool)
                .on_press(Message::CancelJob(job.id)),
        );
    } else {
        header = header.push(
            button(icons::view(icons::CLOSE, 13))
                .padding(4)
                .style(style::tool)
                .on_press(Message::DismissJob(job.id)),
        );
    }

    let mut card = column![
        header,
        progress_bar(0.0..=1.0, job.fraction())
            .length(Fill)
            .girth(Length::Fixed(5.0)),
        text(format!(
            "{} из {} · {} из {} объектов",
            format_size(job.done_bytes),
            format_size(job.total_bytes),
            job.done_items,
            job.total_items
        ))
        .size(11)
        .style(style::muted),
    ]
    .spacing(8);

    if let JobState::Failed(message) = &job.state {
        card = card.push(text(message.clone()).size(12).style(style::danger_text));
    }

    if !job.errors.is_empty() {
        card = card.push(
            text(format!("Ошибок: {}", job.errors.len()))
                .size(12)
                .style(style::danger_text),
        );
        for error in job.errors.iter().take(3) {
            card = card.push(text(error.clone()).size(11).style(style::muted));
        }
    }

    container(card)
        .padding(12)
        .width(Fill)
        .style(|theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(
                theme.extended_palette().background.weakest.color,
            )),
            border: iced::Border {
                radius: style::RADIUS.into(),
                ..iced::Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn error_box<'a>(title: &'a str, message: &'a str) -> Element<'a, Message> {
    let content = column![
        row![
            icons::view(icons::ALERT, 20),
            text(message).size(13).width(Fill),
        ]
        .spacing(12),
        row![
            space().width(Fill),
            button(text("Понятно").size(13))
                .padding([8, 16])
                .style(style::primary)
                .on_press(Message::ModalClose),
        ],
    ]
    .spacing(16);

    frame(title, content.into())
}

/// Пара кнопок «Отмена» / действие.
fn buttons<'a>(
    cancel: &'a str,
    confirm: &'a str,
    confirm_style: fn(&iced::Theme, button::Status) -> button::Style,
    message: Message,
) -> Element<'a, Message> {
    row![
        space().width(Fill),
        button(text(cancel).size(13))
            .padding([8, 16])
            .style(style::neutral)
            .on_press(Message::ModalClose),
        button(text(confirm).size(13))
            .padding([8, 16])
            .style(confirm_style)
            .on_press(message),
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}
