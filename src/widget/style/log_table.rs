use iced::{Background, Border, Color, Theme};
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
pub struct Appearance {
    pub text_color: Color,
    pub background: Background,
    pub border: Border,
    pub header_text_color: Color,
    pub header_background: Background,
    pub header_border: Border,
    pub hovered_text_color: Color,
    pub hovered_background: Background,
    pub selected_text_color: Color,
    pub selected_background: Background,
}

impl std::default::Default for Appearance {
    fn default() -> Self {
        Self {
            text_color: Color::BLACK,
            background: Background::Color([0.87, 0.87, 0.87].into()),
            border: Border {
                width: 1.0,
                color: [0.7, 0.7, 0.7].into(),
                radius: Default::default(),
            },
            header_text_color: Color::WHITE,
            header_background: Background::Color([0.6, 0.6, 0.6].into()),
            header_border: Border::with_radius(1),
            hovered_text_color: Color::WHITE,
            hovered_background: Background::Color([0.0, 0.5, 1.0].into()),
            selected_text_color: Color::WHITE,
            selected_background: Background::Color([0.2, 0.5, 0.8].into()),
        }
    }
}

pub trait StyleSheet {
    type Style: Default + Clone;
    fn style(&self, style: &Self::Style) -> Appearance;
}

#[derive(Clone, Default)]
pub enum LogTableStyles {
    #[default]
    Default,
    Custom(Rc<dyn StyleSheet<Style = Theme>>),
}

impl LogTableStyles {
    pub fn custom(style_sheet: impl StyleSheet<Style = Theme> + 'static) -> Self {
        Self::Custom(Rc::new(style_sheet))
    }
}

impl StyleSheet for Theme {
    type Style = LogTableStyles;
    fn style(&self, style: &Self::Style) -> Appearance {
        if let LogTableStyles::Custom(custom) = style {
            return custom.style(self);
        }

        let palette = self.extended_palette();
        let foreground = self.palette();

        Appearance {
            text_color: foreground.text,
            background: palette.background.base.color.into(),
            border: Border {
                color: foreground.text,
                ..Appearance::default().border
            },
            header_background: palette.primary.weak.color.into(),
            header_border: Border {
                color: foreground.text,
                ..Appearance::default().border
            },
            hovered_text_color: palette.primary.weak.text,
            hovered_background: palette.primary.weak.color.into(),
            selected_text_color: palette.primary.strong.text,
            selected_background: palette.primary.strong.color.into(),
            ..Appearance::default()
        }
    }
}
