//! Верхняя панель: навигация, путь, поиск, режимы отображения, тема.

use std::path::{Component, PathBuf};

use iced::widget::{button, container, row, scrollable, space, text, text_input, tooltip};
use iced::{Center, Element, Fill, Length, Shrink};

use super::{icons, style, App, Message, FILTER_ID, PATH_ID};
use crate::config::ViewMode;
use crate::i18n::S;

/// Высота панели инструментов — по ней позиционируется всплывающее меню тем.
pub const HEIGHT: f32 = 44.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let navigation = row![
        tool(
            icons::ARROW_LEFT,
            app.t(S::Back),
            (app.history_pos > 0).then_some(Message::Back),
        ),
        tool(
            icons::ARROW_RIGHT,
            app.t(S::Forward),
            (app.history_pos + 1 < app.history.len()).then_some(Message::Forward),
        ),
        tool(
            icons::ARROW_UP,
            app.t(S::Up),
            app.dir.parent().map(|_| Message::Up),
        ),
        tool(icons::REFRESH, app.t(S::Refresh), Some(Message::Refresh)),
    ]
    .spacing(2);

    let location: Element<'_, Message> = match &app.path_edit {
        Some(value) => text_input(app.t(S::PathPlaceholder), value)
            .id(PATH_ID)
            .on_input(Message::PathEditChanged)
            .on_submit(Message::PathEditSubmit)
            .padding([6, 10])
            .size(14)
            .style(style::input)
            .width(Fill)
            .into(),
        None => breadcrumbs(app),
    };

    let mut controls = row![
        toggle(
            icons::FOLDER_PLUS,
            app.t(S::NewFolderHint),
            false,
            Message::RequestNewFolder,
        ),
        toggle(
            icons::CHECK,
            if app.is_bookmarked() {
                app.t(S::BookmarkRemove)
            } else {
                app.t(S::BookmarkAdd)
            },
            app.is_bookmarked(),
            Message::ToggleBookmark,
        ),
        toggle(
            icons::SEARCH,
            app.t(S::FilterHint),
            app.filter_visible,
            Message::ToggleFilter,
        ),
        toggle(
            if app.config.show_hidden {
                icons::EYE
            } else {
                icons::EYE_OFF
            },
            app.t(S::HiddenHint),
            app.config.show_hidden,
            Message::ToggleHidden,
        ),
    ]
    .spacing(2)
    .align_y(Center);

    controls = controls.push(space().width(6));

    for mode in ViewMode::ALL {
        let icon = match mode {
            ViewMode::Details => icons::VIEW_DETAILS,
            ViewMode::Compact => icons::VIEW_COMPACT,
            ViewMode::Icons => icons::VIEW_ICONS,
        };
        controls = controls.push(toggle(
            icon,
            app.t(mode.label()),
            app.config.view == mode,
            Message::SetView(mode),
        ));
    }

    // Тема выбирается во всплывающей панели — в строке только иконка.
    controls = controls.push(space().width(6)).push(toggle(
        icons::PALETTE,
        app.t(S::ThemeHint),
        app.theme_menu,
        Message::ToggleThemeMenu,
    ));

    let bar = row![navigation, location, controls]
        .spacing(8)
        .align_y(Center)
        .padding([6, 8]);

    let mut column = iced::widget::column![container(bar)
        .height(Length::Fixed(HEIGHT))
        .style(style::chrome)];

    if app.filter_visible {
        let field = row![
            icons::view(icons::SEARCH, 15),
            text_input(app.t(S::FilterPlaceholder), &app.filter)
                .id(FILTER_ID)
                .on_input(Message::FilterChanged)
                .padding([5, 9])
                .size(13)
                .style(style::input),
            text(app.config.lang.matches(app.filtered.len()))
                .size(12)
                .style(style::muted),
        ]
        .spacing(8)
        .align_y(Center)
        .padding([0, 10]);

        column = column.push(
            container(field)
                .padding([6, 4])
                .width(Fill)
                .style(style::chrome),
        );
    }

    column.push(separator()).into()
}

/// Тонкая горизонтальная линия между панелями.
pub fn separator<'a>() -> Element<'a, Message> {
    container(space())
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(
                theme.extended_palette().background.strong.color,
            )),
            ..container::Style::default()
        })
        .into()
}

fn tool<'a>(icon: &'static str, hint: &'a str, message: Option<Message>) -> Element<'a, Message> {
    let content = button(icons::view(icon, 17))
        .padding(7)
        .style(style::tool)
        .on_press_maybe(message);

    with_hint(content.into(), hint)
}

fn toggle<'a>(
    icon: &'static str,
    hint: &'a str,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    let content = button(icons::view(icon, 17))
        .padding(7)
        .style(if active { style::tool_active } else { style::tool })
        .on_press(message);

    with_hint(content.into(), hint)
}

fn with_hint<'a>(content: Element<'a, Message>, hint: &'a str) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(hint).size(12))
            .padding([4, 8])
            .style(style::popup),
        tooltip::Position::Bottom,
    )
    .gap(4)
    .into()
}

/// Путь в виде кликабельных сегментов. Клик по пустому месту переключает
/// строку в режим ручного ввода — как в классических менеджерах.
fn breadcrumbs(app: &App) -> Element<'_, Message> {
    let mut crumbs = row![].spacing(1).align_y(Center);
    let mut accumulated = PathBuf::new();
    let mut first = true;

    for component in app.dir.components() {
        let label = match component {
            Component::RootDir => {
                accumulated.push("/");
                "/".to_string()
            }
            Component::Normal(name) => {
                accumulated.push(name);
                name.to_string_lossy().to_string()
            }
            other => {
                accumulated.push(other.as_os_str());
                other.as_os_str().to_string_lossy().to_string()
            }
        };

        if !first {
            crumbs = crumbs.push(icons::view(icons::CHEVRON_RIGHT, 13));
        }
        first = false;

        crumbs = crumbs.push(
            button(text(label).size(14))
                .padding([4, 7])
                .style(style::crumb)
                .on_press(Message::NavigateTo(accumulated.clone())),
        );
    }

    let scroller = scrollable(crumbs)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Fill)
        .height(Shrink);

    iced::widget::mouse_area(
        container(scroller)
            .width(Fill)
            .padding([2, 4])
            .style(|theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(
                    theme.extended_palette().background.weakest.color,
                )),
                border: iced::Border {
                    radius: style::RADIUS.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            }),
    )
    .on_press(Message::TogglePathEdit)
    .into()
}
