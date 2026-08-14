//! Основная панель со списком файлов в трёх режимах отображения.
//!
//! Ячейки во всех режимах одинакового размера, поэтому положение элемента
//! считается арифметически ([`Geometry`]) — на этом держится выделение рамкой.

use iced::widget::{button, column, container, mouse_area, row, scrollable, space, stack, text};
use iced::{Center, Element, Fill, Length, Rectangle, Right};

use super::{icons, style, App, Message, LIST_ID, ROW_HEIGHT};
use crate::config::ViewMode;
use crate::fsops::entry::format_time;
use crate::fsops::{Entry, SortKey};
use crate::i18n::S;

/// Отступы содержимого внутри прокручиваемой области.
pub const PAD_X: f32 = 6.0;
pub const PAD_Y: f32 = 4.0;

const LIST_GAP: f32 = 1.0;
const GRID_GAP: f32 = 6.0;
const COMPACT_H: f32 = 26.0;
/// Минимальная ширина ячейки в компактном режиме.
const COMPACT_CELL: f32 = 250.0;
/// Иконка плюс подпись и отступы — высота ячейки в режиме значков.
const ICON_CELL_EXTRA: f32 = 46.0;
/// Полоса прокрутки рисуется поверх содержимого; оставляем ей место.
const SCROLLBAR: f32 = 14.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let empty: Option<Element<'_, Message>> = if let Some(error) = &app.read_error {
        Some(notice(icons::ALERT, app.t(S::DirUnavailable), error))
    } else if app.entries.is_empty() && !app.loading {
        Some(notice(
            icons::FOLDER,
            app.t(S::EmptyTitle),
            app.t(S::EmptyHint),
        ))
    } else if app.filtered.is_empty() && !app.filter.is_empty() {
        Some(notice(
            icons::SEARCH,
            app.t(S::NoMatchTitle),
            app.t(S::NoMatchHint),
        ))
    } else {
        None
    };

    let body: Element<'_, Message> = match empty {
        Some(notice) => mouse_area(container(notice).width(Fill).height(Fill))
            .on_press(Message::EmptyPressed)
            .on_right_press(Message::EmptyRightPressed)
            .into(),
        None => {
            let list = match app.config.view {
                ViewMode::Details => details(app),
                ViewMode::Compact => compact(app),
                ViewMode::Icons => grid_icons(app),
            };

            let scroller = scrollable(container(list).padding([PAD_Y, PAD_X]).width(Fill))
                .id(LIST_ID)
                .height(Fill)
                .width(Fill)
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y));

            // Рамка рисуется поверх списка, в координатах видимой области.
            let mut layers = stack![scroller];
            if let Some(frame) = marquee(app) {
                layers = layers.push(frame);
            }

            mouse_area(layers)
                .on_press(Message::PanelPressed)
                .on_move(Message::PanelMoved)
                .on_release(Message::PanelReleased)
                .on_right_press(Message::EmptyRightPressed)
                .into()
        }
    };

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

// ---------------------------------------------------------------- геометрия

/// Раскладка ячеек списка: одинаковый шаг по обеим осям.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub columns: usize,
    pub cell_w: f32,
    pub cell_h: f32,
    pub gap_x: f32,
    pub gap_y: f32,
    /// Строка занимает всю ширину — по горизонтали рамка не ограничивает.
    pub full_width: bool,
}

impl Geometry {
    pub fn of(app: &App) -> Self {
        let content = content_width(app);

        match app.config.view {
            ViewMode::Details => Self {
                columns: 1,
                cell_w: content,
                cell_h: ROW_HEIGHT,
                gap_x: 0.0,
                gap_y: LIST_GAP,
                full_width: true,
            },
            ViewMode::Compact => {
                let columns = columns_for(content, COMPACT_CELL);
                Self {
                    columns,
                    cell_w: cell_width(content, columns, GRID_GAP),
                    cell_h: COMPACT_H,
                    gap_x: GRID_GAP,
                    gap_y: LIST_GAP,
                    full_width: false,
                }
            }
            ViewMode::Icons => {
                let cell = app.config.icon_size as f32 + ICON_CELL_EXTRA;
                let columns = columns_for(content, cell);
                Self {
                    columns,
                    cell_w: cell_width(content, columns, GRID_GAP),
                    cell_h: cell,
                    gap_x: GRID_GAP,
                    gap_y: GRID_GAP,
                    full_width: false,
                }
            }
        }
    }

    /// Прямоугольник ячейки в координатах содержимого.
    pub fn rect(&self, index: usize) -> Rectangle {
        let row = (index / self.columns) as f32;
        let col = (index % self.columns) as f32;

        Rectangle {
            x: PAD_X + col * (self.cell_w + self.gap_x),
            y: PAD_Y + row * (self.cell_h + self.gap_y),
            width: self.cell_w,
            height: self.cell_h,
        }
    }

    /// Элементы, попавшие в рамку выделения.
    pub fn hits(&self, area: Rectangle, count: usize) -> Vec<usize> {
        let mut out = Vec::new();
        if count == 0 || self.columns == 0 {
            return out;
        }

        let step = self.cell_h + self.gap_y;
        let first = (((area.y - PAD_Y) / step).floor()).max(0.0) as usize;
        let last = (((area.y + area.height - PAD_Y) / step).floor()).max(0.0) as usize;

        let rows = count.div_ceil(self.columns);
        if first >= rows {
            return out;
        }

        for row in first..=last.min(rows - 1) {
            for col in 0..self.columns {
                let index = row * self.columns + col;
                if index >= count {
                    break;
                }

                let cell = self.rect(index);
                let hit = if self.full_width {
                    overlaps_y(&cell, &area)
                } else {
                    overlaps(&cell, &area)
                };

                if hit {
                    out.push(index);
                }
            }
        }

        out
    }
}

fn overlaps(a: &Rectangle, b: &Rectangle) -> bool {
    overlaps_y(a, b) && a.x < b.x + b.width && b.x < a.x + a.width
}

fn overlaps_y(a: &Rectangle, b: &Rectangle) -> bool {
    a.y < b.y + b.height && b.y < a.y + a.height
}

/// Ширина области содержимого без боковой панели, отступов и полосы прокрутки.
pub fn content_width(app: &App) -> f32 {
    (app.window.width - app.config.sidebar_width - 1.0 - PAD_X * 2.0 - SCROLLBAR).max(160.0)
}

fn columns_for(content: f32, cell: f32) -> usize {
    ((content / cell).floor() as usize).max(1)
}

fn cell_width(content: f32, columns: usize, gap: f32) -> f32 {
    let gaps = gap * (columns.saturating_sub(1)) as f32;
    ((content - gaps) / columns as f32).max(1.0)
}

/// Прямоугольник рамки поверх списка.
fn marquee(app: &App) -> Option<Element<'_, Message>> {
    let area = app.marquee.as_ref()?.area()?;

    // Рамка живёт в координатах содержимого, а рисуется поверх видимой части.
    let x = area.x;
    let y = area.y - app.scroll;

    Some(
        super::pinned(
            container(space())
                .width(Length::Fixed(area.width))
                .height(Length::Fixed(area.height))
                .style(|theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        background: Some(iced::Background::Color(style::faded(
                            palette.primary.base.color,
                            0.18,
                        ))),
                        border: iced::Border {
                            color: palette.primary.base.color,
                            width: 1.0,
                            radius: 2.0.into(),
                        },
                        ..container::Style::default()
                    }
                })
                .into(),
            iced::Point::new(x, y),
        ),
    )
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

    let mut label = row![text(app.t(key.label())).size(12).style(if active {
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
    let mut list = column![].spacing(LIST_GAP);

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
                app.lang().size(entry.size)
            })
            .size(12)
            .width(Length::Fixed(96.0))
            .align_x(Right)
            .style(style::muted),
            text(app.t(entry.kind.label()))
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
    let geometry = Geometry::of(app);
    let mut grid = column![].spacing(LIST_GAP);
    let mut current = row![].spacing(GRID_GAP);
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
                    .height(Length::Fixed(COMPACT_H))
                    .width(Fill)
                    .into(),
            ))
            .width(Fill),
        );

        in_row += 1;
        if in_row == geometry.columns {
            grid = grid.push(current);
            current = row![].spacing(GRID_GAP);
            in_row = 0;
        }
    }

    if in_row > 0 {
        // Хвост дополняем пустотой, иначе последние ячейки растянутся.
        for _ in in_row..geometry.columns {
            current = current.push(container(space()).width(Fill));
        }
        grid = grid.push(current);
    }

    grid.into()
}

// ---------------------------------------------------------------- «Значки»

fn grid_icons(app: &App) -> Element<'_, Message> {
    let geometry = Geometry::of(app);

    let mut grid = column![].spacing(GRID_GAP);
    let mut current = row![].spacing(GRID_GAP);
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
                // Высота фиксирована: от неё считается попадание рамки.
                container(cell)
                    .width(Fill)
                    .height(Length::Fixed(geometry.cell_h))
                    .padding([10, 6])
                    .center_x(Fill)
                    .clip(true)
                    .into(),
            ))
            .width(Fill),
        );

        in_row += 1;
        if in_row == geometry.columns {
            grid = grid.push(current);
            current = row![].spacing(GRID_GAP);
            in_row = 0;
        }
    }

    if in_row > 0 {
        for _ in in_row..geometry.columns {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn list_geometry() -> Geometry {
        Geometry {
            columns: 1,
            cell_w: 600.0,
            cell_h: 30.0,
            gap_x: 0.0,
            gap_y: 1.0,
            full_width: true,
        }
    }

    fn grid_geometry() -> Geometry {
        Geometry {
            columns: 3,
            cell_w: 100.0,
            cell_h: 100.0,
            gap_x: 6.0,
            gap_y: 6.0,
            full_width: false,
        }
    }

    fn area(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn ячейки_идут_с_постоянным_шагом() {
        let geometry = list_geometry();
        assert_eq!(geometry.rect(0).y, PAD_Y);
        assert_eq!(geometry.rect(1).y, PAD_Y + 31.0);
        assert_eq!(geometry.rect(2).y, PAD_Y + 62.0);

        let grid = grid_geometry();
        assert_eq!(grid.rect(3).x, PAD_X);
        assert_eq!(grid.rect(4).x, PAD_X + 106.0);
        assert_eq!(grid.rect(4).y, PAD_Y + 106.0);
    }

    #[test]
    fn рамка_в_списке_берёт_строки_по_вертикали() {
        let geometry = list_geometry();

        // рамка накрывает первые две строки
        let hits = geometry.hits(area(0.0, 0.0, 10.0, 40.0), 10);
        assert_eq!(hits, vec![0, 1]);

        // узкая рамка у левого края всё равно выделяет строку целиком
        let hits = geometry.hits(area(0.0, 40.0, 2.0, 5.0), 10);
        assert_eq!(hits, vec![1]);
    }

    #[test]
    fn рамка_не_цепляет_промежутки_между_строками() {
        let geometry = list_geometry();
        // зазор между первой и второй строкой: 34..35
        let hits = geometry.hits(area(0.0, 34.2, 10.0, 0.5), 10);
        assert!(hits.is_empty(), "получено {hits:?}");
    }

    #[test]
    fn рамка_в_сетке_учитывает_столбцы() {
        let geometry = grid_geometry();
        // столбцы занимают 6..106, 112..212, 218..318

        // рамка обрывается на 110 — во второй столбец не дотягивается
        let hits = geometry.hits(area(10.0, 10.0, 100.0, 20.0), 9);
        assert_eq!(hits, vec![0]);

        // до 160 — попадают оба столбца первого ряда
        let hits = geometry.hits(area(10.0, 10.0, 150.0, 20.0), 9);
        assert_eq!(hits, vec![0, 1]);

        // прямоугольник через два ряда и два столбца
        let hits = geometry.hits(area(10.0, 10.0, 150.0, 150.0), 9);
        assert_eq!(hits, vec![0, 1, 3, 4]);
    }

    #[test]
    fn рамка_не_выходит_за_число_элементов() {
        let geometry = grid_geometry();
        let hits = geometry.hits(area(0.0, 0.0, 1000.0, 1000.0), 5);
        assert_eq!(hits, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn пустой_список_и_рамка_выше_содержимого() {
        let geometry = list_geometry();
        assert!(geometry.hits(area(0.0, 0.0, 100.0, 100.0), 0).is_empty());
        assert!(geometry.hits(area(0.0, 900.0, 100.0, 50.0), 3).is_empty());
    }
}
