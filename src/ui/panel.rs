//! Основная панель со списком файлов в трёх режимах отображения.

use iced::widget::{button, column, container, mouse_area, row, scrollable, space, text};
use iced::{Center, Element, Fill, Length, Right};

use super::{icons, style, App, Message, LIST_ID, ROW_HEIGHT};
use crate::config::ViewMode;
use crate::fsops::entry::{format_size, format_time};
use crate::fsops::{Entry, SortKey};

pub fn view(app: &App) -> Element<'_, Message> {
    let content: Element<'_, Message> = if let Some(error) = &app.read_error {
        notice(icons::ALERT, "Каталог недоступен", error)
    } else if app.entries.is_empty() && !app.loading {
        notice(icons::FOLDER, "Здесь пусто", "В этой папке нет файлов")
    } else if app.filtered.is_empty() && !app.filter.is_empty() {
        notice(
            icons::SEARCH,
            "Ничего не найдено",
            "Измените условие фильтра",
        )
    } else {
        let list = match app.config.view {
            ViewMode::Details => details(app),
            ViewMode::Compact => compact(app),
            ViewMode::Icons => grid_icons(app),
        };

        scrollable(container(list).padding([4, 6]).width(Fill))
            .id(LIST_ID)
            .height(Fill)
            .width(Fill)
            .into()
    };

    let body = mouse_area(container(content).width(Fill).height(Fill))
        .on_press(Message::EmptyPressed)
        .on_right_press(Message::EmptyRightPressed);

    let mut panel = column![];
    if app.config.view == ViewMode::Details && app.read_error.is_none() {
        panel = panel.push(details_header(app));
    }
    panel = panel.push(body);

    container(panel)
        .width(Fill)
        .height(Fill)
        .style(style::panel)
        .into()
}

// -------------------------------------------------------------- «Подробно»

fn details_header(app: &App) -> Element<'_, Message> {
    let header = row![
        space().width(Length::Fixed(28.0)),
        sort_button(app, SortKey::Name, Fill.into()),
        sort_button(app, SortKey::Size, Length::Fixed(96.0)),
        sort_button(app, SortKey::Kind, Length::Fixed(128.0)),
        sort_button(app, SortKey::Modified, Length::Fixed(150.0)),
    ]
    .spacing(10)
    .padding([0, 16])
    .align_y(Center);

    column![
        container(header)
            .height(Length::Fixed(30.0))
            .width(Fill)
            .style(style::header),
        super::toolbar::separator(),
    ]
    .into()
}

fn sort_button(app: &App, key: SortKey, width: Length) -> Element<'_, Message> {
    let active = app.config.sort_key == key;

    let mut label = row![text(key.label()).size(12).style(if active {
        style::accent_text
    } else {
        style::muted
    })]
    .spacing(4)
    .align_y(Center);

    if active {
        let icon = if app.config.sort_ascending {
            icons::SORT_ASC
        } else {
            icons::SORT_DESC
        };
        label = label.push(icons::view(icon, 11));
    }

    button(label)
        .width(width)
        .padding([4, 4])
        .style(style::crumb)
        .on_press(Message::SetSort(key))
        .into()
}

fn details(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(1);

    for (index, entry) in app.visible_entries() {
        let cells = row![
            icons::colored(
                icons::for_kind(entry.kind),
                18,
                style::kind_color(&app.theme, entry.kind)
            ),
            name_cell(app, entry, Fill),
            text(if entry.is_dir {
                String::new()
            } else {
                format_size(entry.size)
            })
            .size(12)
            .width(Length::Fixed(96.0))
            .align_x(Right)
            .style(style::muted),
            text(entry.kind.label())
                .size(12)
                .width(Length::Fixed(128.0))
                .style(style::muted),
            text(
                entry
                    .modified
                    .map(format_time)
                    .unwrap_or_else(|| "—".into())
            )
            .size(12)
            .width(Length::Fixed(150.0))
            .style(style::muted),
        ]
        .spacing(10)
        .align_y(Center)
        .padding([0, 10]);

        list = list.push(row_wrapper(
            app,
            entry,
            index,
            container(cells)
                .height(Length::Fixed(ROW_HEIGHT))
                .width(Fill)
                .into(),
        ));
    }

    list.into()
}

// ------------------------------------------------------------- «Компактно»

fn compact(app: &App) -> Element<'_, Message> {
    let columns = columns_for(app, 250.0);
    let mut grid = column![].spacing(1);
    let mut current = row![].spacing(6);
    let mut in_row = 0;

    for (index, entry) in app.visible_entries() {
        let cells = row![
            icons::colored(
                icons::for_kind(entry.kind),
                16,
                style::kind_color(&app.theme, entry.kind)
            ),
            name_cell(app, entry, Fill),
        ]
        .spacing(8)
        .align_y(Center)
        .padding([0, 8]);

        current = current.push(
            container(row_wrapper(
                app,
                entry,
                index,
                container(cells)
                    .height(Length::Fixed(26.0))
                    .width(Fill)
                    .into(),
            ))
            .width(Fill),
        );

        in_row += 1;
        if in_row == columns {
            grid = grid.push(current);
            current = row![].spacing(6);
            in_row = 0;
        }
    }

    if in_row > 0 {
        // Хвост дополняем пустотой, иначе последние ячейки растянутся.
        for _ in in_row..columns {
            current = current.push(container(space()).width(Fill));
        }
        grid = grid.push(current);
    }

    grid.into()
}

// ---------------------------------------------------------------- «Значки»

fn grid_icons(app: &App) -> Element<'_, Message> {
    let size = app.config.icon_size as f32;
    let columns = columns_for(app, size + 44.0);

    let mut grid = column![].spacing(6);
    let mut current = row![].spacing(6);
    let mut in_row = 0;

    for (index, entry) in app.visible_entries() {
        let cell = column![
            icons::colored(
                icons::for_kind(entry.kind),
                app.config.icon_size,
                style::kind_color(&app.theme, entry.kind)
            ),
            text(shorten(&entry.name, 30))
                .size(12)
                .align_x(Center)
                .style(if cut_out(app, entry) {
                    style::muted
                } else {
                    default_text
                }),
        ]
        .spacing(8)
        .align_x(Center);

        current = current.push(
            container(row_wrapper(
                app,
                entry,
                index,
                container(cell)
                    .width(Fill)
                    .padding([10, 6])
                    .center_x(Fill)
                    .into(),
            ))
            .width(Fill),
        );

        in_row += 1;
        if in_row == columns {
            grid = grid.push(current);
            current = row![].spacing(6);
            in_row = 0;
        }
    }

    if in_row > 0 {
        for _ in in_row..columns {
            current = current.push(container(space()).width(Fill));
        }
        grid = grid.push(current);
    }

    grid.into()
}

// ----------------------------------------------------------------- детали

/// Обёртка строки: выделение, наведение, клики.
fn row_wrapper<'a>(
    app: &'a App,
    entry: &'a Entry,
    index: usize,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    let selected = app.selection.contains(&entry.path);
    let hovered = app.hover == Some(index);

    mouse_area(
        container(content)
            .width(Fill)
            .style(style::row(selected, hovered)),
    )
    .on_press(Message::RowPressed(index))
    .on_double_click(Message::Activate(index))
    .on_right_press(Message::RowRightPressed(index))
    .on_enter(Message::RowHovered(Some(index)))
    .on_exit(Message::RowHovered(None))
    .into()
}

fn name_cell<'a>(app: &'a App, entry: &'a Entry, width: Length) -> Element<'a, Message> {
    let mut line = row![text(&entry.name)
        .size(13)
        .wrapping(text::Wrapping::None)
        .style(if entry.is_broken_link {
            style::danger_text
        } else if cut_out(app, entry) {
            style::muted
        } else {
            default_text
        })]
    .spacing(5)
    .align_y(Center);

    if entry.is_symlink {
        line = line.push(icons::view(icons::LINK, 12));
    }

    container(line).width(width).clip(true).into()
}

/// Элемент «вырезан» в буфер обмена — показываем его приглушённо.
fn cut_out(app: &App, entry: &Entry) -> bool {
    match &app.clipboard {
        Some((crate::ipc::Op::Move, paths)) => paths.contains(&entry.path),
        _ => false,
    }
}

fn default_text(theme: &iced::Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().background.base.text),
    }
}

/// Сколько колонок помещается в панель при заданной ширине ячейки.
fn columns_for(app: &App, cell: f32) -> usize {
    let available = (app.window.width - app.config.sidebar_width - 40.0).max(220.0);
    ((available / cell).floor() as usize).max(1)
}

fn shorten(name: &str, limit: usize) -> String {
    if name.chars().count() <= limit {
        return name.to_string();
    }
    let head: String = name.chars().take(limit - 1).collect();
    format!("{head}…")
}

fn notice<'a>(icon: &'static str, title: &'a str, description: &'a str) -> Element<'a, Message> {
    container(
        column![
            icons::view(icon, 44),
            text(title).size(16),
            text(description).size(13).style(style::muted),
        ]
        .spacing(10)
        .align_x(Center),
    )
    .width(Fill)
    .height(Fill)
    .center_x(Fill)
    .center_y(Fill)
    .into()
}
