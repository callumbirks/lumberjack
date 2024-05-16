use crate::widget::log_table::list::ListState;
use crate::widget::log_table::{Content, Mutables};
use crate::widget::style::log_table::StyleSheet;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{renderer, text, Layout, Widget};
use iced::alignment::{Horizontal, Vertical};
use iced::mouse::Cursor;
use iced::{Element, Length, Pixels, Point, Rectangle, Size};
use std::hash::Hash;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub(super) struct ColumnView<'a, T, Message, Theme, Renderer>
where
    T: Clone,
    Renderer: renderer::Renderer + text::Renderer,
    Theme: StyleSheet,
{
    pub index: usize,
    pub content: &'a Content<T>,
    pub mutables: Arc<RwLock<Mutables<T, Message, Renderer>>>,
    pub style: Theme::Style,
    pub padding: f32,
    pub text_size: f32,
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ColumnView<'a, T, Message, Theme, Renderer>
where
    T: Clone + Hash,
    Renderer: renderer::Renderer + text::Renderer,
    Theme: StyleSheet,
{
    fn size(&self) -> Size<Length> {
        let width = {
            let Ok(mutables) = self.mutables.read() else {
                return Size::new(Length::Shrink, Length::Shrink);
            };
            *mutables
                .column_widths
                .get(self.index)
                .unwrap_or(&Length::Shrink)
        };
        Size::new(width, Length::Shrink)
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits.width(self.size().width).height(Length::Fill);

        let intrinsic = Size::new(
            limits.max().width,
            (self.text_size + self.padding * 2.0) * (self.content.rows.len() + 1) as f32,
        );

        Node::new(intrinsic)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let list_state = tree.state.downcast_ref::<ListState>();

        let bounds = layout.bounds();

        let (font, width) = {
            let Ok(mutables) = self.mutables.read() else {
                return;
            };
            (
                mutables.font,
                *mutables
                    .column_widths
                    .get(self.index)
                    .unwrap_or(&Length::Shrink),
            )
        };

        let item_height = self.text_size + (self.padding * 2.0);
        let offset = viewport.y - bounds.y;
        let start = (offset / item_height) as u64;
        let end = ((offset + viewport.height) / item_height).ceil() as u64;

        let appearance = theme.style(&self.style);

        let column = &self.content.columns[self.index];

        let bounds = Rectangle {
            height: item_height,
            ..bounds
        };

        renderer.fill_text(
            text::Text {
                content: column.title,
                bounds: bounds.size(),
                size: Pixels(self.text_size),
                line_height: Default::default(),
                font,
                horizontal_alignment: Horizontal::Left,
                vertical_alignment: Vertical::Top,
                shaping: text::Shaping::Basic,
            },
            Point::new(bounds.x + self.padding, bounds.y + self.padding),
            appearance.header_text_color,
            bounds,
        );

        let end = end.saturating_sub(1);

        for i in start..end.min(self.content.rows.len() as u64) {
            let is_selected = list_state.last_selected_index.is_some_and(|u| u.0 == i);
            let is_hovered = list_state.hovered_option == Some(i);

            let bounds = Rectangle {
                y: bounds.y + item_height * (i + 1) as f32,
                ..bounds
            };

            let text_color = if is_selected {
                appearance.selected_text_color
            } else if is_hovered {
                appearance.hovered_text_color
            } else {
                appearance.text_color
            };

            renderer.fill_text(
                text::Text {
                    content: &self.content.rows[i as usize].cells[self.index],
                    bounds: Size::new(f32::INFINITY, bounds.height),
                    size: Pixels(self.text_size),
                    line_height: Default::default(),
                    font,
                    horizontal_alignment: Horizontal::Left,
                    vertical_alignment: Vertical::Top,
                    shaping: text::Shaping::Basic,
                },
                Point::new(bounds.x + self.padding, bounds.y),
                text_color,
                bounds,
            );
        }
    }
}

impl<'a, T, Message, Theme, Renderer> From<ColumnView<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: StyleSheet + 'a,
    Renderer: renderer::Renderer + text::Renderer + 'a,
    T: Clone + Hash,
{
    fn from(value: ColumnView<'a, T, Message, Theme, Renderer>) -> Self {
        Self::new(value)
    }
}
