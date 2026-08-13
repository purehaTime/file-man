//! Левая панель: быстрый доступ и список дисков.

use iced::widget::{column, container, mouse_area, progress_bar, row, scrollable, space, text};
use iced::{Center, Element, Fill, Length};

use super::{icons, style, App, Message};
use crate::fsops::Place;
use crate::i18n::S;

pub fn view(app: &App) -> Element<'_, Message> {
    let mut content = column![].spacing(2).padding([8, 8]);

    content = content.push(heading(app.t(S::QuickAccess)));
    for (index, place) in app.quick.iter().enumerate() {
        content = content.push(item(app, place, index));
    }

    if !app.drives.is_empty() {
        content = content.push(space().height(10));
        content = content.push(heading(app.t(S::Devices)));

        let offset = app.quick.len();
        for (index, place) in app.drives.iter().enumerate() {
            content = content.push(item(app, place, offset + index));
        }
    }

    let panel = container(scrollable(content).height(Fill))
        .width(Length::Fixed(app.config.sidebar_width))
        .height(Fill)
        .style(style::sidebar);

    row![panel, vertical_separator()].into()
}

fn heading<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(11).style(style::muted))
        .padding([6, 8])
        .into()
}

fn item<'a>(app: &'a App, place: &'a Place, index: usize) -> Element<'a, Message> {
    let active = app.dir == place.path;
    let hovered = app.hover_place == Some(index);

    let mut body = column![row![
        icons::view(icons::for_place(place.kind), 16),
        text(&place.label).size(13).wrapping(text::Wrapping::None),
    ]
    .spacing(9)
    .align_y(Center)];

    // У дисков показываем заполненность — это ожидаемо от файлового менеджера.
    if let Some((free, total)) = place.usage {
        if total > 0 {
            let used = (total - free) as f32 / total as f32;

            body = body.push(
                column![
                    progress_bar(0.0..=1.0, used)
                        .girth(Length::Fixed(3.0))
                        .style(move |theme: &iced::Theme| {
                            let palette = theme.extended_palette();
                            let bar = if used > 0.9 {
                                palette.danger.base.color
                            } else {
                                palette.primary.base.color
                            };
                            progress_bar::Style {
                                background: iced::Background::Color(
                                    palette.background.strong.color,
                                ),
                                bar: iced::Background::Color(bar),
                                border: iced::Border {
                                    radius: 2.0.into(),
                                    ..iced::Border::default()
                                },
                            }
                        }),
                    text(app.lang().free_space(free))
                        .size(10)
                        .style(style::muted),
                ]
                .spacing(3)
                .padding([2, 25]),
            );
        }
    }

    mouse_area(
        container(body)
            .width(Fill)
            .padding([6, 9])
            .style(style::place(active, hovered)),
    )
    .on_press(Message::NavigateTo(place.path.clone()))
    .on_enter(Message::PlaceHovered(Some(index)))
    .on_exit(Message::PlaceHovered(None))
    .into()
}

fn vertical_separator<'a>() -> Element<'a, Message> {
    container(space())
        .width(Length::Fixed(1.0))
        .height(Fill)
        .style(|theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(
                theme.extended_palette().background.strong.color,
            )),
            ..container::Style::default()
        })
        .into()
}
