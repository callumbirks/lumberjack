mod list;

use crate::widget::log_table::list::List;
use crate::widget::style::log_table::StyleSheet;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::text::Paragraph;
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{graphics, layout, renderer, text, Clipboard, Layout, Shell, Widget};
use iced::alignment::{Horizontal, Vertical};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::{container, scrollable, Container, Scrollable};
use iced::{Alignment, Event, Font, Length, Padding, Pixels, Rectangle, Shadow, Size};
use std::hash::{Hash, Hasher};

pub struct LogTable<'a, T, Message, Theme, Renderer>
where
    T: Hash + Clone,
    Renderer: renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: StyleSheet + container::StyleSheet,
{
    content: &'a Content<T>,
    container: Container<'a, Message, Theme, Renderer>,
    font: Renderer::Font,
    width: Length,
    height: Length,
    max_width: f32,
    padding: f32,
    text_size: f32,
    style: <Theme as StyleSheet>::Style,
}

impl<'a, T, Message, Theme, Renderer> LogTable<'a, T, Message, Theme, Renderer>
where
    T: Hash + Clone,
    Renderer: 'a + renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: 'a + StyleSheet + container::StyleSheet + scrollable::StyleSheet,
    Message: 'a,
{
    pub fn with_content(
        content: &'a Content<T>,
        on_click: impl Fn(usize, T) -> Message + 'static,
    ) -> LogTable<'a, T, Message, Theme, Renderer> {
        let container = Container::new(Scrollable::new(List {
            content,
            font: Font::default(),
            text_size: 12.0,
            padding: 5.0,
            style: <Theme as StyleSheet>::Style::default(),
            on_click: Box::new(on_click),
            selected: None,
        }))
        .padding(1);

        LogTable {
            content,
            container,
            font: Font::default(),
            width: Length::Fill,
            height: Length::Fill,
            max_width: 0.0,
            padding: 5.0,
            text_size: 12.0,
            style: <Theme as StyleSheet>::Style::default(),
        }
    }
}

// log_table::Content::new_with(["Line", "Timestamp", "Object"], &items, |item| {
//      log_table::Row::new_with(item, [item.line, item.timestamp, item.object])
// })
// .on_click(|item| Message::ItemSelected)

pub struct Content<T>
where
    T: Clone,
{
    columns: Box<[Column]>,
    rows: Box<[Row<T>]>,
}

impl<T> Content<T>
where
    T: Clone,
{
    pub fn new_empty() -> Content<T> {
        let columns: Box<[Column]> = Box::new([]);
        let rows: Box<[Row<T>]> = Box::new([]);
        Content { columns, rows }
    }

    pub fn new_with(
        columns: &[&'static str],
        items: &[T],
        row_builder: impl Fn(&T) -> Row<T>,
    ) -> Content<T> {
        let columns: Box<[Column]> = columns.iter().map(|title| Column { title }).collect();
        let rows: Box<[Row<T>]> = items.iter().map(row_builder).collect();
        Content { columns, rows }
    }
}

struct Column {
    title: &'static str,
}

pub struct Row<T>
where
    T: Clone,
{
    item: T,
    cells: Box<[Box<str>]>,
}

impl<T> Row<T>
where
    T: Clone,
{
    pub fn new_with(item: &T, cells: &[impl ToString]) -> Row<T> {
        Row {
            item: item.clone(),
            cells: cells
                .iter()
                .map(ToString::to_string)
                .map(String::into_boxed_str)
                .collect(),
        }
    }
}

impl<T> Hash for Row<T>
where
    T: Clone + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.item.hash(state)
    }
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for LogTable<'a, T, Message, Theme, Renderer>
where
    T: Hash + Clone,
    Renderer: renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: StyleSheet + container::StyleSheet,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let limits = limits.width(self.width).height(self.height);
        let max_width = match self.width {
            Length::Shrink => self
                .content
                .rows
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let text_str = item.cells.iter().fold(String::new(), |mut string, cell| {
                        string.push_str(cell);
                        string
                    });
                    let text = text::Text {
                        content: &text_str,
                        bounds: Size::INFINITY,
                        size: Pixels(self.text_size),
                        line_height: Default::default(),
                        font: self.font,
                        horizontal_alignment: Horizontal::Left,
                        vertical_alignment: Vertical::Top,
                        shaping: text::Shaping::Basic,
                    };

                    state.values[index].update(text);
                    state.values[index].min_bounds().width.round() as u32 + self.padding as u32 * 2
                })
                .max()
                .unwrap_or(100),
            _ => limits.max().width as u32,
        };

        let limits = limits.max_width(max_width as f32 + self.padding * 2.0);

        let content = self
            .container
            .layout(&mut tree.children[0], renderer, &limits);
        let size = limits.resolve(self.width, self.height, content.size());
        Node::with_children(size, vec![content])
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
        let Some(_clipped_viewport) = layout.bounds().intersection(viewport) else {
            return;
        };

        let appearance = theme.style(&self.style);

        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border: appearance.border,
                shadow: Shadow::default(),
            },
            appearance.background,
        );

        self.container.draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout
                .children()
                .next()
                .expect("Scrollable Child Missing in Log Table"),
            cursor,
            &layout.bounds(),
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.container as &dyn Widget<_, _, _>)]
    }

    fn diff(&self, _tree: &mut Tree) {
        _tree.diff_children(&[&self.container as &dyn Widget<_, _, _>]);
        let state = _tree.state.downcast_mut::<State>();

        state.values = self
            .content
            .rows
            .iter()
            .map(|_| graphics::text::Paragraph::new())
            .collect();
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> Status {
        self.container.on_event(
            &mut state.children[0],
            event,
            layout
                .children()
                .next()
                .expect("Scrollable Child Missing in Log Table"),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> Interaction {
        self.container
            .mouse_interaction(&state.children[0], layout, cursor, viewport, renderer)
    }
}

pub struct State {
    values: Vec<graphics::text::Paragraph>,
}

impl State {
    pub fn new<T: Clone>(rows: &[Row<T>]) -> Self {
        Self {
            values: rows
                .iter()
                .map(|_| graphics::text::Paragraph::new())
                .collect(),
        }
    }
}
