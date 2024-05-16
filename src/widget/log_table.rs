mod column;
mod list;

use crate::widget::log_table::column::ColumnView;
use crate::widget::log_table::list::List;
use crate::widget::style::log_table::StyleSheet;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::text::Paragraph;
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{graphics, renderer, text, Clipboard, Layout, Shell, Widget};
use iced::alignment::{Horizontal, Vertical};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::{container, scrollable, Container, Scrollable};
use iced::{Element, Event, Length, Pixels, Rectangle, Shadow, Size};
use lazy_static::lazy_static;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

pub struct LogTable<'a, T, Message, Theme, Renderer>
where
    T: Hash + Clone,
    Renderer: renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: StyleSheet + container::StyleSheet,
{
    content: &'a Content<T>,
    container: Container<'a, Message, Theme, Renderer>,
    mutables: Arc<RwLock<Mutables<T, Message, Renderer>>>,
    width: Length,
    height: Length,
    padding: f32,
    text_size: f32,
    style: <Theme as StyleSheet>::Style,
}

lazy_static! {
    static ref SCROLLABLE_ID: scrollable::Id = scrollable::Id::unique();
}

impl<'a, T, Message, Theme, Renderer> LogTable<'a, T, Message, Theme, Renderer>
where
    T: Hash + Clone,
    Renderer: 'a + renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: 'a + StyleSheet + container::StyleSheet + scrollable::StyleSheet,
    Message: 'static,
{
    pub fn with_content(
        content: &'a Content<T>,
        on_scroll: impl Fn(scrollable::Viewport) -> Message + 'static,
    ) -> LogTable<'a, T, Message, Theme, Renderer> {
        let mutables = Arc::new(RwLock::new(Mutables {
            on_click: None,
            column_widths: Box::default(),
            font: iced::Font::default(),
        }));

        let columns = content
            .columns
            .iter()
            .enumerate()
            .map(|(i, _)| {
                ColumnView {
                    index: i,
                    content,
                    mutables: mutables.clone(),
                    style: <Theme as StyleSheet>::Style::default(),
                    padding: 5.0,
                    text_size: 12.0,
                }
                .into()
            })
            .collect();

        let list = List {
            content,
            columns,
            mutables: mutables.clone(),
            text_size: 12.0,
            padding: 5.0,
            style: <Theme as StyleSheet>::Style::default(),
            selected: None,
        };

        let id = SCROLLABLE_ID.clone();

        let container = Container::new(Scrollable::new(list).id(id).on_scroll(on_scroll));

        LogTable {
            content,
            mutables,
            container,
            width: Length::Fill,
            height: Length::Fill,
            padding: 5.0,
            text_size: 12.0,
            style: <Theme as StyleSheet>::Style::default(),
        }
    }

    pub fn on_click_row<F>(self, f: F) -> Self
    where
        F: Fn(u64, T) -> Message + 'static,
    {
        {
            let Ok(mut mutables) = self.mutables.write() else {
                return self;
            };
            mutables.on_click = Some(Box::new(f));
        }
        self
    }

    pub fn column_widths<I>(self, widths: I) -> Self
    where
        I: IntoIterator<Item = Length>,
    {
        {
            let Ok(mut mutables) = self.mutables.write() else {
                return self;
            };
            mutables.column_widths = widths.into_iter().collect();
        }
        self
    }

    pub fn font(self, font: Renderer::Font) -> Self {
        {
            let Ok(mut mutables) = self.mutables.write() else {
                return self;
            };
            mutables.font = font;
        }
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }
}

pub(super) struct Mutables<T, Message, Renderer>
where
    Renderer: text::Renderer,
{
    on_click: Option<Box<dyn Fn(u64, T) -> Message + 'static>>,
    column_widths: Box<[Length]>,
    font: Renderer::Font,
}

// log_table::Content::new_with(["Line", "Timestamp", "Object"], &items, |item| {
//      log_table::Row::new_with(item, [item.line, item.timestamp, item.object])
// })
// .on_click(|item| Message::ItemSelected)

#[derive(Clone)]
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

    pub fn new_with<C>(columns: C, items: &[T], row_builder: impl Fn(&T) -> Row<T>) -> Content<T>
    where
        C: IntoIterator<Item = &'static str>,
    {
        let columns: Box<[Column]> = columns.into_iter().map(|title| Column { title }).collect();
        let rows: Box<[Row<T>]> = items.iter().map(row_builder).collect();
        Content { columns, rows }
    }

    pub fn focus_line<Message: 'static>(&self, line: u64) -> iced::Command<Message> {
        let row_count = self.rows.len();
        let middle_row = row_count as u64 / 2;
        let centering_offset = (line as f64 - middle_row as f64) / (row_count as f64 / 50.0);
        let offset = scrollable::RelativeOffset {
            x: 0.0,
            // The `1.0 +` accounts for column headers
            y: ((1.0 + line as f64 + centering_offset) / row_count as f64) as f32,
        };
        scrollable::snap_to(SCROLLABLE_ID.clone(), offset)
    }
}

#[derive(Debug, Clone)]
struct Column {
    title: &'static str,
}

#[derive(Clone)]
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
    pub fn new_with<I>(item: &T, cells: I) -> Row<T>
    where
        I: IntoIterator<Item = String>,
    {
        Row {
            item: item.clone(),
            cells: cells.into_iter().map(String::into_boxed_str).collect(),
        }
    }
}

impl<T> Debug for Content<T>
where
    T: Debug + Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("Content");
        builder.field("columns", &self.columns);
        builder.field("rows", &self.rows);
        builder.finish()
    }
}

impl<T> Debug for Row<T>
where
    T: Debug + Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("Row");
        builder.field("item", &self.item);
        builder.field("cells", &self.cells);
        builder.finish()
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
        let font = {
            let Ok(mutables) = self.mutables.read() else {
                return Node::default();
            };
            mutables.font
        };
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
                        font,
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

    fn state(&self) -> tree::State {
        tree::State::new(State::new(&self.content.rows))
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

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.container
                .operate(&mut state.children[0], layout, renderer, operation);
        });
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
impl<'a, T, Message, Theme, Renderer> From<LogTable<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Clone + Hash,
    Message: 'a,
    Renderer: 'a + renderer::Renderer + text::Renderer<Font = iced::Font>,
    Theme: 'a + StyleSheet + container::StyleSheet,
{
    fn from(list: LogTable<'a, T, Message, Theme, Renderer>) -> Self {
        Element::new(list)
    }
}
