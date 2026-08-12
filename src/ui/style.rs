//! Оформление: всё берётся из палитры текущей темы, поэтому переключение
//! Catppuccin (и остальных) работает без единого захардкоженного цвета.

use iced::widget::{button, container, text_input};
use iced::{Background, Border, Color, Theme};

use crate::fsops::Kind;

pub const RADIUS: f32 = 6.0;

pub fn mix(a: Color, b: Color, factor: f32) -> Color {
    let f = factor.clamp(0.0, 1.0);
    Color {
        r: a.r * (1.0 - f) + b.r * f,
        g: a.g * (1.0 - f) + b.g * f,
        b: a.b * (1.0 - f) + b.b * f,
        a: a.a * (1.0 - f) + b.a * f,
    }
}

pub fn faded(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

/// Левая панель.
pub fn sidebar(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        ..container::Style::default()
    }
}

/// Панель инструментов и статусная строка.
pub fn chrome(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weaker.color)),
        ..container::Style::default()
    }
}

/// Основная область со списком файлов.
pub fn panel(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        ..container::Style::default()
    }
}

/// Шапка таблицы в режиме «Подробно».
pub fn header(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// Всплывающая панель: контекстное меню, диалоги.
pub fn popup(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weaker.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: (RADIUS + 2.0).into(),
        },
        shadow: iced::Shadow {
            color: Color { a: 0.35, ..Color::BLACK },
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 24.0,
        },
        ..container::Style::default()
    }
}

/// Затемнение под модальным окном.
pub fn backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color {
            a: 0.45,
            ..Color::BLACK
        })),
        ..container::Style::default()
    }
}

/// Строка списка файлов.
pub fn row(selected: bool, hovered: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let palette = theme.extended_palette();

        let background = if selected {
            Some(faded(palette.primary.base.color, 0.28))
        } else if hovered {
            Some(palette.background.weak.color)
        } else {
            None
        };

        container::Style {
            background: background.map(Background::Color),
            border: Border {
                color: if selected {
                    faded(palette.primary.base.color, 0.65)
                } else {
                    Color::TRANSPARENT
                },
                width: if selected { 1.0 } else { 0.0 },
                radius: RADIUS.into(),
            },
            ..container::Style::default()
        }
    }
}

/// Элемент левой панели.
pub fn place(active: bool, hovered: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let palette = theme.extended_palette();

        let background = if active {
            Some(faded(palette.primary.base.color, 0.32))
        } else if hovered {
            Some(palette.background.weak.color)
        } else {
            None
        };

        container::Style {
            background: background.map(Background::Color),
            border: Border {
                radius: RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        }
    }
}

/// Кнопка панели инструментов: без фона, подсветка при наведении.
pub fn tool(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let background = match status {
        button::Status::Hovered => Some(palette.background.weak.color),
        button::Status::Pressed => Some(palette.background.neutral.color),
        _ => None,
    };

    button::Style {
        background: background.map(Background::Color),
        text_color: match status {
            button::Status::Disabled => faded(palette.background.base.text, 0.35),
            _ => palette.background.base.text,
        },
        border: Border {
            radius: RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Кнопка панели инструментов во включённом состоянии.
pub fn tool_active(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let base = faded(palette.primary.base.color, 0.30);
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            faded(palette.primary.base.color, 0.42)
        }
        _ => base,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.background.base.text,
        border: Border {
            radius: RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Сегмент «хлебных крошек».
pub fn crumb(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(palette.background.weak.color)),
            button::Status::Pressed => Some(Background::Color(palette.background.neutral.color)),
            _ => None,
        },
        text_color: palette.background.base.text,
        border: Border {
            radius: (RADIUS - 2.0).into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Пункт меню.
pub fn menu_item(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    button::Style {
        background: match status {
            button::Status::Hovered | button::Status::Pressed => {
                Some(Background::Color(faded(palette.primary.base.color, 0.30)))
            }
            _ => None,
        },
        text_color: match status {
            button::Status::Disabled => faded(palette.background.base.text, 0.35),
            _ => palette.background.base.text,
        },
        border: Border {
            radius: (RADIUS - 2.0).into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Основная кнопка диалога.
pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let background = match status {
        button::Status::Hovered => palette.primary.strong.color,
        button::Status::Pressed => palette.primary.weak.color,
        button::Status::Disabled => faded(palette.primary.base.color, 0.4),
        button::Status::Active => palette.primary.base.color,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.primary.base.text,
        border: Border {
            radius: RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Опасное действие: удаление.
pub fn danger(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let background = match status {
        button::Status::Hovered => palette.danger.strong.color,
        button::Status::Pressed => palette.danger.weak.color,
        button::Status::Disabled => faded(palette.danger.base.color, 0.4),
        button::Status::Active => palette.danger.base.color,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.danger.base.text,
        border: Border {
            radius: RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Вторичная кнопка диалога.
pub fn neutral(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let background = match status {
        button::Status::Hovered => palette.background.strong.color,
        button::Status::Pressed => palette.background.stronger.color,
        _ => palette.background.neutral.color,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.background.base.text,
        border: Border {
            radius: RADIUS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Поля ввода: чуть темнее фона, скруглённые.
pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.extended_palette();

    let active = matches!(status, text_input::Status::Focused { .. });

    text_input::Style {
        background: Background::Color(palette.background.weakest.color),
        border: Border {
            color: if active {
                palette.primary.base.color
            } else {
                palette.background.strong.color
            },
            width: 1.0,
            radius: RADIUS.into(),
        },
        icon: faded(palette.background.base.text, 0.6),
        placeholder: faded(palette.background.base.text, 0.45),
        value: palette.background.base.text,
        selection: faded(palette.primary.base.color, 0.4),
    }
}

/// Цвет иконки по типу файла — оттенки берутся из палитры темы.
pub fn kind_color(theme: &Theme, kind: Kind) -> Color {
    let palette = theme.extended_palette();
    let text = palette.background.base.text;

    match kind {
        Kind::Folder => palette.primary.base.color,
        Kind::Image => palette.success.base.color,
        Kind::Video => mix(palette.primary.base.color, palette.danger.base.color, 0.5),
        Kind::Audio => palette.warning.base.color,
        Kind::Archive => mix(palette.warning.base.color, palette.danger.base.color, 0.4),
        Kind::Code => mix(palette.primary.base.color, palette.success.base.color, 0.5),
        Kind::Pdf => palette.danger.base.color,
        Kind::Document => mix(text, palette.primary.base.color, 0.35),
        Kind::Text => faded(text, 0.8),
        Kind::Executable => palette.success.strong.color,
        Kind::Unknown => faded(text, 0.65),
    }
}

/// Приглушённый текст (размер, дата, подсказки).
pub fn muted(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(faded(theme.extended_palette().background.base.text, 0.62)),
    }
}

pub fn accent_text(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().primary.base.color),
    }
}

pub fn danger_text(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().danger.base.color),
    }
}
