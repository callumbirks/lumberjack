use crate::widget::log_table::Content;
use crate::widget::style::log_table::StyleSheet;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::widget::tree;
use iced::advanced::widget::Tree;
use iced::advanced::{renderer, text, Clipboard, Layout, Shell, Widget};
use iced::alignment::{Horizontal, Vertical};
use iced::{
    event, mouse, touch, Border, Color, Element, Event, Length, Pixels, Point, Rectangle, Shadow,
    Size,
};
use std::hash::{DefaultHasher, Hash, Hasher};

pub struct List<'a, T, Message, Theme, Renderer>
where
    T: Clone + Hash,
    Renderer: renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: StyleSheet,
{
    pub content: &'a Content<T>,
    pub font: Renderer::Font,
    pub style: Theme::Style,
    pub on_click: Box<dyn Fn(usize, T) -> Message + 'static>,
    pub padding: f32,
    pub text_size: f32,
    pub selected: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub hovered_option: Option<usize>,
    pub last_selected_index: Option<(usize, u64)>,
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

    fn layout(&self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits.height(Length::Fill).width(Length::Fill);

        let intrinsic = Size::new(
            limits.max().width,
            (self.text_size + self.padding * 2.0) * self.content.rows.len() as f32,
        );

        Node::new(intrinsic)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let item_height = self.text_size + (self.padding * 2.0);
        let offset = viewport.y - bounds.y;
        let start = (offset / item_height) as usize;
        let end = ((offset + viewport.height) / item_height).ceil() as usize;
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

        // Column header titles
        for (i, column) in self.content.columns.iter().enumerate() {
            let header_bounds = Rectangle {
                x: header_row_bounds.x + 20.0 * i as f32,
                width: 20.0,
                ..header_row_bounds
            };

            renderer.fill_text(
                text::Text {
                    content: column.title,
                    bounds: header_bounds.size(),
                    size: Pixels(self.text_size),
                    line_height: Default::default(),
                    font: self.font,
                    horizontal_alignment: Horizontal::Left,
                    vertical_alignment: Vertical::Top,
                    shaping: text::Shaping::Basic,
                },
                Point::new(header_bounds.x, header_bounds.center_y()),
                appearance.header_text_color,
                header_bounds,
            )
        }

        // Take one off the rows we will draw to account for the header
        let end = end - 1;

        // Visible rows
        for i in start..end.min(self.content.rows.len()) {
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

            let text_color = if is_selected {
                appearance.selected_text_color
            } else if is_hovered {
                appearance.hovered_text_color
            } else {
                appearance.text_color
            };

            for (i, cell) in self.content.rows[i].cells.iter().enumerate() {
                let cell_bounds = Rectangle {
                    x: bounds.x + 20.0 * i as f32,
                    ..bounds
                };

                renderer.fill_text(
                    text::Text {
                        content: cell,
                        bounds: Size::new(f32::INFINITY, bounds.height),
                        size: Pixels(self.text_size),
                        line_height: Default::default(),
                        font: self.font,
                        horizontal_alignment: Horizontal::Left,
                        vertical_alignment: Vertical::Top,
                        shaping: text::Shaping::Basic,
                    },
                    Point::new(bounds.x, bounds.center_y()),
                    text_color,
                    cell_bounds,
                );
            }
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ListState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ListState::default())
    }

    fn diff(&self, tree: &mut Tree) {
        let list_state = tree.state.downcast_mut::<ListState>();

        if let Some(idx) = self.selected {
            if let Some(row) = self.content.rows.get(idx) {
                let mut hasher = DefaultHasher::new();
                row.hash(&mut hasher);

                list_state.last_selected_index = Some((idx, hasher.finish()));
            } else {
                list_state.last_selected_index = None;
            }
        } else if let Some((idx, hash)) = list_state.last_selected_index {
            if let Some(row) = self.content.rows.get(idx) {
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

        if bounds.contains(cursor) {
            match event {
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    list_state.hovered_option = Some(
                        ((cursor.y - bounds.y) / (self.text_size + (self.padding * 2.0))) as usize,
                    );
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. }) => {
                    list_state.hovered_option = Some(
                        ((cursor.y - bounds.y) / (self.text_size + (self.padding * 2.0))) as usize,
                    );

                    if let Some(index) = list_state.hovered_option {
                        if let Some(row) = self.content.rows.get(index) {
                            let mut hasher = DefaultHasher::new();
                            row.hash(&mut hasher);
                            list_state.last_selected_index = Some((index, hasher.finish()));
                        }
                    }

                    status =
                        list_state
                            .last_selected_index
                            .map_or(event::Status::Ignored, |last| {
                                if let Some(row) = self.content.rows.get(last.0) {
                                    _shell.publish((self.on_click)(last.0, row.item.clone()));
                                    event::Status::Captured
                                } else {
                                    event::Status::Ignored
                                }
                            });
                }
                _ => {}
            }
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
