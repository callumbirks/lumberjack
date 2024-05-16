use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, RwLock};

use iced::advanced::layout::flex::Axis;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::widget::tree;
use iced::advanced::widget::Tree;
use iced::advanced::{layout, renderer, text, Clipboard, Layout, Shell, Widget};
use iced::keyboard::key::Named;
use iced::{
    event, keyboard, mouse, touch, Alignment, Background, Border, Color, Element, Event, Length,
    Padding, Rectangle, Shadow, Size,
};

use crate::widget::log_table::{Content, Mutables};
use crate::widget::style::log_table::StyleSheet;

pub struct List<'a, T, Message, Theme, Renderer>
where
    T: Clone + Hash,
    Renderer: renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: StyleSheet,
{
    pub content: &'a Content<T>,
    pub columns: Box<[Element<'a, Message, Theme, Renderer>]>,
    pub mutables: Arc<RwLock<Mutables<T, Message, Renderer>>>,
    pub style: Theme::Style,
    pub padding: f32,
    pub text_size: f32,
    pub selected: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub hovered_option: Option<u64>,
    pub last_selected_index: Option<(u64, u64)>,
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for List<'a, T, Message, Theme, Renderer>
where
    T: Clone + Hash,
    Renderer: renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: StyleSheet,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits.height(Length::Fill).width(Length::Fill);

        //let intrinsic = Size::new(
        //    limits.max().width,
        //    (self.text_size + self.padding * 2.0) * (self.content.rows.len() + 1) as f32,
        //);

        //let children = self
        //    .columns
        //    .iter()
        //    .zip(&mut tree.children)
        //    .map(|(c, tree)| c.layout(tree, renderer, &limits))
        //    .collect();

        //Node::with_children(intrinsic, children)

        let height = (self.text_size + self.padding * 2.0) * (self.content.rows.len() + 1) as f32;

        layout::flex::resolve(
            Axis::Horizontal,
            renderer,
            &limits,
            Length::Fill,
            Length::Fixed(height),
            Padding::ZERO,
            0.0,
            Alignment::Start,
            &self.columns,
            &mut tree.children,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        let item_height = self.text_size + (self.padding * 2.0);
        let offset = viewport.y - bounds.y;
        let start = (offset / item_height) as u64;
        let end = ((offset + viewport.height) / item_height).ceil() as u64;
        let list_state = tree.state.downcast_ref::<ListState>();

        let appearance = theme.style(&self.style);

        let header_row_bounds = Rectangle {
            height: self.text_size + (self.padding * 2.0),
            ..bounds
        };

        // Header row
        renderer.fill_quad(
            renderer::Quad {
                bounds: header_row_bounds,
                border: appearance.header_border,
                shadow: Shadow::default(),
            },
            appearance.header_background,
        );

        // Take one off the rows we will draw to account for the header
        let end = end.saturating_sub(1);

        // Visible rows
        for i in start..end.min(self.content.rows.len() as u64) {
            let is_selected = list_state.last_selected_index.is_some_and(|u| u.0 == i);
            let is_hovered = list_state.hovered_option == Some(i);

            let bounds = Rectangle {
                x: bounds.x,
                y: bounds.y + item_height * (i + 1) as f32,
                width: bounds.width,
                height: self.text_size + (self.padding * 2.0),
            };

            if is_selected || is_hovered {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        border: Border {
                            radius: (0.0).into(),
                            width: 0.0,
                            color: Color::TRANSPARENT,
                        },
                        shadow: Shadow::default(),
                    },
                    if is_selected {
                        appearance.selected_background
                    } else {
                        appearance.hovered_background
                    },
                );
            }
        }

        for (column, layout) in self.columns.iter().zip(layout.children()) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: appearance.border,
                    shadow: Shadow::default(),
                },
                Background::Color(Color::TRANSPARENT),
            );
            column
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ListState>()
    }

    fn children(&self) -> Vec<Tree> {
        self.columns
            .iter()
            .map(|c| Tree::new(c.as_widget()))
            .collect()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ListState::default())
    }

    fn diff(&self, tree: &mut Tree) {
        let children = self
            .columns
            .iter()
            .map(|c| c.as_widget())
            .collect::<Box<[_]>>();
        tree.diff_children(&children);

        let list_state = tree.state.downcast_mut::<ListState>();

        if let Some(idx) = self.selected {
            if let Some(row) = self.content.rows.get(idx as usize) {
                let mut hasher = DefaultHasher::new();
                row.hash(&mut hasher);

                list_state.last_selected_index = Some((idx, hasher.finish()));
            } else {
                list_state.last_selected_index = None;
            }
        } else if let Some((idx, hash)) = list_state.last_selected_index {
            if let Some(row) = self.content.rows.get(idx as usize) {
                let mut hasher = DefaultHasher::new();
                row.hash(&mut hasher);

                if hash != hasher.finish() {
                    list_state.last_selected_index = None;
                }
            } else {
                list_state.last_selected_index = None;
            }
        }
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        _shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let bounds = layout.bounds();
        let mut status = event::Status::Ignored;
        let list_state = tree.state.downcast_mut::<ListState>();
        let cursor = cursor.position().unwrap_or_default();

        let Some(mutables) = self.mutables.read().ok() else {
            return event::Status::Ignored;
        };

        let Some(on_click) = &mutables.on_click else {
            return event::Status::Ignored;
        };

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) if bounds.contains(cursor) => {
                list_state.hovered_option = Some(
                    ((cursor.y - bounds.y - (self.text_size + (self.padding * 2.0)))
                        / (self.text_size + (self.padding * 2.0))) as u64,
                );
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
                if bounds.contains(cursor) =>
            {
                list_state.hovered_option = Some(
                    ((cursor.y - bounds.y - (self.text_size + (self.padding * 2.0)))
                        / (self.text_size + (self.padding * 2.0))) as u64,
                );

                if let Some(index) = list_state.hovered_option {
                    if let Some(row) = self.content.rows.get(index as usize) {
                        let mut hasher = DefaultHasher::new();
                        row.hash(&mut hasher);
                        list_state.last_selected_index = Some((index, hasher.finish()));
                    }
                }

                status = list_state
                    .last_selected_index
                    .map_or(event::Status::Ignored, |last| {
                        if let Some(row) = self.content.rows.get(last.0 as usize) {
                            _shell.publish(on_click(last.0, row.item.clone()));
                            event::Status::Captured
                        } else {
                            event::Status::Ignored
                        }
                    });
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                status = match key {
                    keyboard::Key::Named(Named::ArrowUp) => {
                        if let Some((last_selected, _)) = list_state.last_selected_index {
                            let selected = last_selected
                                .wrapping_sub(1)
                                .min(self.content.rows.len() as u64);
                            let hash = self.hash_row_at(selected).unwrap_or(0);
                            list_state.last_selected_index = Some((selected, hash));
                            if let Some(row) = self.row(selected) {
                                _shell.publish(on_click(selected, row.item.clone()))
                            }
                            event::Status::Captured
                        } else {
                            event::Status::Ignored
                        }
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        if let Some((last_selected, _)) = list_state.last_selected_index {
                            let selected = last_selected.saturating_add(1);
                            let selected = if selected >= self.content.rows.len() as u64 {
                                0
                            } else {
                                selected
                            };
                            let hash = self.hash_row_at(selected).unwrap_or(0);
                            list_state.last_selected_index = Some((selected, hash));
                            if let Some(row) = self.row(selected) {
                                _shell.publish(on_click(selected, row.item.clone()))
                            }
                            event::Status::Captured
                        } else {
                            event::Status::Ignored
                        }
                    }
                    _ => event::Status::Ignored,
                }
            }
            _ => {}
        }

        status
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();

        if bounds.contains(cursor.position().unwrap_or_default()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, T, Message, Theme, Renderer> List<'a, T, Message, Theme, Renderer>
where
    T: Clone + Hash,
    Renderer: renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: StyleSheet,
{
    pub fn item_height(&self) -> f32 {
        self.text_size + (self.padding * 2.0)
    }

    fn hash_row_at(&self, index: u64) -> Option<u64> {
        if let Some(row) = self.content.rows.get(index as usize) {
            let mut hasher = DefaultHasher::new();
            row.hash(&mut hasher);
            Some(hasher.finish())
        } else {
            None
        }
    }

    pub fn row(&self, index: u64) -> Option<&super::Row<T>> {
        self.content.rows.get(index as usize)
    }
}

impl<'a, T, Message, Theme, Renderer> From<List<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Clone + Hash,
    Message: 'a,
    Renderer: 'a + renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: 'a + StyleSheet,
{
    fn from(list: List<'a, T, Message, Theme, Renderer>) -> Self {
        Element::new(list)
    }
}
