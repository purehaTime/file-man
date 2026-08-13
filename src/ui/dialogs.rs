//! Контекстное меню, панель выбора темы и модальные окна.

use iced::widget::{
    button, column, container, mouse_area, opaque, progress_bar, row, scrollable, space, text,
    text_input,
};
use iced::{Center, Color, Element, Fill, Length, Size};

use super::{clamp_menu, icons, pinned, style, App, Context, Message, Modal, DIALOG_ID};
use crate::config::ThemeChoice;
use crate::fsops::entry::format_time;
use crate::fsops::ops;
use crate::i18n::S;
use crate::ipc::{Job, JobState};

const MENU_WIDTH: f32 = 250.0;
const MENU_ITEM: f32 = 30.0;
const THEME_WIDTH: f32 = 250.0;

// ------------------------------------------------------------ меню по ПКМ

pub fn context_menu<'a>(app: &'a App, context: &'a Context) -> Element<'a, Message> {
    let on_entry = context.target.is_some();
    let can_paste = app.clipboard.is_some() && ops::is_writable(&app.dir);
    let selected = app.selection.len();

    let mut items: Vec<Element<'a, Message>> = Vec::new();
    let mut height = 12.0;

    if on_entry {
        items.push(entry_item(
            icons::PLAY,
            app.t(S::Open),
            Some(Message::OpenSelected),
        ));
        items.push(divider());
        items.push(entry_item(
            icons::COPY,
            app.t(S::Copy),
            Some(Message::CopySelection),
        ));
        items.push(entry_item(
            icons::CUT,
            app.t(S::Cut),
            Some(Message::CutSelection),
        ));
        height += MENU_ITEM * 3.0 + 9.0;
    }

    items.push(entry_item(
        icons::PASTE,
        app.t(S::Paste),
        can_paste.then_some(Message::Paste),
    ));
    height += MENU_ITEM;

    if on_entry {
        items.push(divider());
        items.push(entry_item(
            icons::RENAME,
            app.t(S::Rename),
            (selected == 1).then_some(Message::RequestRename),
        ));
        items.push(entry_item(
            icons::TRASH,
            app.t(S::MoveToTrash),
            Some(Message::RequestDelete { permanent: false }),
        ));
        items.push(entry_item(
            icons::CLOSE,
            app.t(S::DeleteForever),
            Some(Message::RequestDelete { permanent: true }),
        ));
        height += MENU_ITEM * 3.0 + 9.0;
    }

    items.push(divider());
    items.push(entry_item(
        icons::FOLDER_PLUS,
        app.t(S::NewFolder),
        Some(Message::RequestNewFolder),
    ));
    items.push(entry_item(
        icons::FILE,
        app.t(S::NewFile),
        Some(Message::RequestNewFile),
    ));
    height += MENU_ITEM * 2.0 + 9.0;

    if on_entry {
        items.push(divider());
        items.push(entry_item(
            icons::INFO,
            app.t(S::Properties),
            (selected == 1).then_some(Message::RequestProperties),
        ));
        height += MENU_ITEM + 9.0;
    }

    let menu = container(column(items).spacing(1))
        .width(Length::Fixed(MENU_WIDTH))
        .padding(6)
        .style(style::popup);

    let position = clamp_menu(app, context.position, Size::new(MENU_WIDTH, height));

    overlay(menu.into(), position, Message::ContextClose)
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

/// Слой поверх интерфейса: клик мимо закрывает всплывающую панель.
fn overlay<'a>(
    content: Element<'a, Message>,
    position: iced::Point,
    close: Message,
) -> Element<'a, Message> {
    iced::widget::stack![
        mouse_area(container(space()).width(Fill).height(Fill))
            .on_press(close.clone())
            .on_right_press(close),
        pinned(content, position),
    ]
    .into()
}

// -------------------------------------------------------------- выбор темы

/// Компактная панель тем: открывается по иконке в панели инструментов.
pub fn theme_menu(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(1);

    for choice in ThemeChoice::ALL {
        let active = app.config.theme == choice;

        // Цвета берём из самой темы — получается наглядный образец.
        let theme = choice.theme();
        let palette = theme.extended_palette();
        let swatch = [
            palette.background.base.color,
            palette.primary.base.color,
            palette.success.base.color,
        ];

        let mut line = row![
            dots(swatch),
            text(choice.name(app.lang())).size(13).width(Fill),
        ]
        .spacing(10)
        .align_y(Center);

        if active {
            line = line.push(icons::view(icons::CHECK, 13));
        }

        list = list.push(
            button(line)
                .width(Fill)
                .padding([6, 8])
                .style(style::menu_item)
                .on_press(Message::SetTheme(choice)),
        );
    }

    // Список длинный — при низком окне прокручиваем.
    let max_height = (app.window.height - super::toolbar::HEIGHT - 40.0).max(200.0);
    let body = column![
        text(app.t(S::ThemeTitle)).size(12).style(style::muted),
        scrollable(list).height(Length::Shrink),
    ]
    .spacing(8);

    let menu = container(body)
        .width(Length::Fixed(THEME_WIDTH))
        .max_height(max_height)
        .padding(8)
        .style(style::popup);

    let position = iced::Point::new(
        (app.window.width - THEME_WIDTH - 10.0).max(8.0),
        super::toolbar::HEIGHT + 4.0,
    );

    overlay(menu.into(), position, Message::ToggleThemeMenu)
}

/// Три кружка с цветами темы.
fn dots<'a>(colors: [Color; 3]) -> Element<'a, Message> {
    let mut line = row![].spacing(3).align_y(Center);

    for color in colors {
        line = line.push(
            container(space())
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(10.0))
                .style(move |theme: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(color)),
                    border: iced::Border {
                        color: theme.extended_palette().background.strong.color,
                        width: 1.0,
                        radius: 5.0.into(),
                    },
                    ..container::Style::default()
                }),
        );
    }

    line.into()
}

// ------------------------------------------------------------ модальные окна

pub fn modal<'a>(app: &'a App, modal: &'a Modal) -> Element<'a, Message> {
    let body = match modal {
        Modal::Rename { value, .. } => prompt(app, S::Rename, S::NewNameLabel, value),
        Modal::NewFolder { value } => prompt(app, S::NewFolder, S::FolderNameLabel, value),
        Modal::NewFile { value } => prompt(app, S::NewFile, S::FileNameLabel, value),
        Modal::ConfirmDelete { paths, permanent } => confirm_delete(app, paths, *permanent),
        Modal::Properties { entry, contents } => properties(app, entry, *contents),
        Modal::Jobs => jobs_list(app),
        Modal::Error { title, message } => error_box(app, title, message),
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

fn prompt<'a>(app: &'a App, title: S, placeholder: S, value: &'a str) -> Element<'a, Message> {
    let content = column![
        text_input(app.t(placeholder), value)
            .id(DIALOG_ID)
            .on_input(Message::ModalInput)
            .on_submit(Message::ModalSubmit)
            .padding([8, 10])
            .size(14)
            .style(style::input),
        buttons(
            app,
            S::DialogCancel,
            S::DialogApply,
            style::primary,
            Message::ModalSubmit,
        ),
    ]
    .spacing(16);

    frame(app.t(title), content.into())
}

fn confirm_delete<'a>(
    app: &'a App,
    paths: &'a [std::path::PathBuf],
    permanent: bool,
) -> Element<'a, Message> {
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
            text(app.lang().and_more(paths.len() - 8))
                .size(13)
                .style(style::muted),
        );
    }

    let content = column![
        text(app.t(if permanent {
            S::DeleteWarning
        } else {
            S::TrashWarning
        }))
        .size(13),
        list,
        buttons(
            app,
            S::DialogCancel,
            if permanent {
                S::DeleteConfirm
            } else {
                S::MoveToTrash
            },
            if permanent { style::danger } else { style::primary },
            Message::ModalSubmit,
        ),
    ]
    .spacing(14);

    frame(
        app.t(if permanent {
            S::DeleteTitle
        } else {
            S::TrashTitle
        }),
        content.into(),
    )
}

fn properties<'a>(
    app: &'a App,
    entry: &'a crate::fsops::Entry,
    contents: Option<usize>,
) -> Element<'a, Message> {
    let mut rows = column![
        field(app.t(S::FieldName), entry.name.clone()),
        field(app.t(S::FieldPath), entry.path.display().to_string()),
        field(app.t(S::FieldKind), app.t(entry.kind.label()).to_string()),
    ]
    .spacing(8);

    if entry.is_dir {
        if let Some(count) = contents {
            rows = rows.push(field(
                app.t(S::FieldContents),
                app.lang().items(count as u64),
            ));
        }
    } else {
        rows = rows.push(field(
            app.t(S::FieldSize),
            format!("{} ({})", app.lang().size(entry.size), entry.size),
        ));
    }

    if let Some(modified) = entry.modified {
        rows = rows.push(field(app.t(S::FieldModified), format_time(modified)));
    }
    if entry.hidden {
        rows = rows.push(field(app.t(S::FieldHidden), app.t(S::Yes).to_string()));
    }
    if entry.is_symlink {
        let target = std::fs::read_link(&entry.path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "—".into());
        rows = rows.push(field(app.t(S::FieldLinkTarget), target));
    }

    let content = column![
        rows,
        row![
            space().width(Fill),
            button(text(app.t(S::DialogClose)).size(13))
                .padding([8, 16])
                .style(style::neutral)
                .on_press(Message::ModalClose),
        ],
    ]
    .spacing(16);

    frame(app.t(S::Properties), content.into())
}

fn field<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    row![
        text(label)
            .size(12)
            .width(Length::Fixed(110.0))
            .style(style::muted),
        text(value).size(13).width(Fill),
    ]
    .spacing(10)
    .into()
}

fn jobs_list(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(10);

    if app.jobs.is_empty() {
        list = list.push(text(app.t(S::JobsEmpty)).size(13).style(style::muted));
    }

    for job in &app.jobs {
        list = list.push(job_card(app, job));
    }

    let content = column![
        scrollable(list).height(Length::Fixed(320.0)),
        row![
            button(text(app.t(S::JobsClearDone)).size(13))
                .padding([8, 14])
                .style(style::neutral)
                .on_press(Message::DismissFinished),
            space().width(Fill),
            button(text(app.t(S::DialogClose)).size(13))
                .padding([8, 16])
                .style(style::primary)
                .on_press(Message::ModalClose),
        ]
        .spacing(10)
        .align_y(Center),
    ]
    .spacing(16);

    frame(app.t(S::JobsTitle), content.into())
}

fn job_card<'a>(app: &'a App, job: &'a Job) -> Element<'a, Message> {
    let mut header = row![
        text(super::statusbar::job_title(app, job)).size(13).width(Fill),
        text(app.t(super::statusbar::state_label(&job.state)))
            .size(12)
            .style(style::muted),
    ]
    .spacing(10)
    .align_y(Center);

    header = header.push(
        button(icons::view(icons::CLOSE, 13))
            .padding(4)
            .style(style::tool)
            .on_press(if job.state.is_active() {
                Message::CancelJob(job.id)
            } else {
                Message::DismissJob(job.id)
            }),
    );

    let mut card = column![
        header,
        progress_bar(0.0..=1.0, job.fraction())
            .length(Fill)
            .girth(Length::Fixed(5.0)),
        text(app.lang().job_amounts(
            job.done_bytes,
            job.total_bytes,
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
            text(app.lang().errors(job.errors.len()))
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

fn error_box<'a>(app: &'a App, title: &'a str, message: &'a str) -> Element<'a, Message> {
    let content = column![
        row![
            icons::view(icons::ALERT, 20),
            text(message).size(13).width(Fill),
        ]
        .spacing(12),
        row![
            space().width(Fill),
            button(text(app.t(S::DialogOk)).size(13))
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
    app: &'a App,
    cancel: S,
    confirm: S,
    confirm_style: fn(&iced::Theme, button::Status) -> button::Style,
    message: Message,
) -> Element<'a, Message> {
    row![
        space().width(Fill),
        button(text(app.t(cancel)).size(13))
            .padding([8, 16])
            .style(style::neutral)
            .on_press(Message::ModalClose),
        button(text(app.t(confirm)).size(13))
            .padding([8, 16])
            .style(confirm_style)
            .on_press(message),
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}
