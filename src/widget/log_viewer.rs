use iced::advanced::graphics::text::cosmic_text::LayoutRun;
use iced::advanced::layout::Limits;
use iced::advanced::renderer::Style;
use iced::advanced::text::editor::Cursor;
use iced::advanced::text::{highlighter, Editor, Highlighter, LineHeight, Paragraph};
use iced::advanced::{
    clipboard, layout, mouse, renderer, text, widget, Clipboard, Layout, Shell, Text, Widget,
};
use iced::alignment::{Horizontal, Vertical};
use iced::futures::SinkExt;
use iced::keyboard::key;
use iced::widget::text_editor::StyleSheet;
use iced::{
    event, keyboard, Element, Event, Length, Padding, Pixels, Rectangle, Renderer, Size, Vector,
};
use std::cell::RefCell;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};

pub use text::editor::{Action, Motion};

pub fn log_viewer<Message, Theme, Renderer>(
    content: &Content<Renderer>,
) -> LogViewer<'_, highlighter::PlainText, Message, Theme, Renderer>
where
    Theme: StyleSheet,
    Renderer: text::Renderer,
{
    LogViewer::new(content)
}

pub struct LogViewer<'a, Highlighter, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Highlighter: text::Highlighter,
    Theme: StyleSheet,
    Renderer: text::Renderer,
{
    content: &'a Content<Renderer>,
    font: Option<Renderer::Font>,
    line_numbers_width: f32,
    text_size: Option<Pixels>,
    line_height: LineHeight,
    width: Length,
    height: Length,
    padding: Padding,
    style: Theme::Style,
    on_edit: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    highlighter_settings: Highlighter::Settings,
    highlighter_format: fn(&Highlighter::Highlight, &Theme) -> highlighter::Format<Renderer::Font>,
}

impl<'a, Message, Theme, Renderer> LogViewer<'a, highlighter::PlainText, Message, Theme, Renderer>
where
    Theme: StyleSheet,
    Renderer: text::Renderer,
{
    pub fn new(content: &'a Content<Renderer>) -> Self {
        Self {
            content,
            font: None,
            line_numbers_width: 70.0,
            text_size: None,
            line_height: LineHeight::default(),
            width: Length::Fill,
            height: Length::Shrink,
            padding: Padding::new(5.0),
            style: Default::default(),
            on_edit: None,
            highlighter_settings: (),
            highlighter_format: |_highlight, _theme| highlighter::Format::default(),
        }
    }
}

impl<'a, Highlighter, Message, Theme, Renderer> LogViewer<'a, Highlighter, Message, Theme, Renderer>
where
    Highlighter: text::Highlighter,
    Theme: StyleSheet,
    Renderer: text::Renderer,
{
    pub fn line_numbers(mut self, width: f32) -> Self {
        self.line_numbers_width = width;
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn on_action(mut self, on_edit: impl Fn(Action) -> Message + 'a) -> Self {
        self.on_edit = Some(Box::new(on_edit));
        self
    }

    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn highlight<H: text::Highlighter>(
        self,
        settings: H::Settings,
        to_format: fn(&H::Highlight, &Theme) -> highlighter::Format<Renderer::Font>,
    ) -> LogViewer<'a, H, Message, Theme, Renderer> {
        LogViewer {
            content: self.content,
            font: self.font,
            line_numbers_width: self.line_numbers_width,
            text_size: self.text_size,
            line_height: self.line_height,
            width: self.width,
            height: self.height,
            padding: self.padding,
            style: self.style,
            on_edit: self.on_edit,
            highlighter_settings: settings,
            highlighter_format: to_format,
        }
    }

    pub fn style(mut self, style: impl Into<Theme::Style>) -> Self {
        self.style = style.into();
        self
    }

    fn editor_bounds(&self, layout: Layout) -> Rectangle {
        layout.bounds() + Vector::new(self.line_numbers_width, 0.0)
    }

    fn line_numbers_str(editor: &iced::advanced::graphics::text::Editor) -> Vec<String> {
        let buffer = editor.buffer();
        let line_numbers = buffer.layout_runs().map(|r| r.line_i).collect::<Vec<_>>();
        line_numbers.iter().map(usize::to_string).collect()
    }
}

pub struct Content<R = iced::Renderer>(RefCell<Internal<R>>)
where
    R: text::Renderer;

struct Internal<R>
where
    R: text::Renderer,
{
    editor: R::Editor,
}

impl<R> Content<R>
where
    R: text::Renderer,
{
    pub fn new() -> Self {
        Self::with_text("")
    }

    pub fn with_text(text: &str) -> Self {
        Self(RefCell::new(Internal {
            editor: Editor::with_text(text),
        }))
    }

    pub fn perform(&mut self, action: Action) {
        let internal = self.0.get_mut();
        internal.editor.perform(action);
    }

    pub fn line_count(&self) -> usize {
        self.0.borrow().editor.line_count()
    }

    pub fn line(&self, index: usize) -> Option<impl std::ops::Deref<Target = str> + '_> {
        std::cell::Ref::filter_map(self.0.borrow(), |internal| internal.editor.line(index)).ok()
    }

    pub fn lines(&self) -> impl Iterator<Item = impl std::ops::Deref<Target = str> + '_> {
        struct Lines<'a, Renderer: text::Renderer> {
            internal: std::cell::Ref<'a, Internal<Renderer>>,
            current: usize,
        }

        impl<'a, Renderer: text::Renderer> Iterator for Lines<'a, Renderer> {
            type Item = std::cell::Ref<'a, str>;

            fn next(&mut self) -> Option<Self::Item> {
                let line =
                    std::cell::Ref::filter_map(std::cell::Ref::clone(&self.internal), |internal| {
                        internal.editor.line(self.current)
                    })
                    .ok()?;
                self.current += 1;
                Some(line)
            }
        }

        Lines {
            internal: self.0.borrow(),
            current: 0,
        }
    }

    pub fn text(&self) -> String {
        let mut text = self
            .lines()
            .enumerate()
            .fold(String::new(), |mut contents, (i, line)| {
                if i > 0 {
                    contents.push('\n');
                }
                contents.push_str(&line);
                contents
            });
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text
    }

    pub fn selection(&self) -> Option<String> {
        self.0.borrow().editor.selection()
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        self.0.borrow().editor.cursor_position()
    }
}

impl<R> Content<R>
where
    R: text::Renderer<Editor = iced::advanced::graphics::text::Editor>,
{
    pub fn set_line(&mut self, index: i32) {
        let mut internal = self.0.borrow_mut();
        let half_lines = internal.editor.buffer().visible_lines() / 2;
        internal.editor.perform(Action::Scroll {
            lines: index - half_lines,
        });
    }
}

impl<Renderer> Default for Content<Renderer>
where
    Renderer: text::Renderer,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Renderer> fmt::Debug for Content<Renderer>
where
    Renderer: text::Renderer,
    Renderer::Editor: fmt::Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let internal = self.0.borrow();
        f.debug_struct("Content")
            .field("editor", &internal.editor)
            .finish()
    }
}

unsafe impl<R: text::Renderer> Send for Content<R> {}
unsafe impl<R: text::Renderer> Sync for Content<R> {}

struct State<Highlighter: text::Highlighter> {
    is_focused: bool,
    last_click: Option<mouse::Click>,
    drag_click: Option<mouse::click::Kind>,
    partial_scroll: f32,
    highlighter: RefCell<Highlighter>,
    highlighter_settings: Highlighter::Settings,
    highlighter_format_address: usize,
}

impl<'a, Highlighter, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for LogViewer<'a, Highlighter, Message, Theme, Renderer>
where
    Highlighter: text::Highlighter,
    Theme: StyleSheet,
    Renderer: text::Renderer<Editor = iced::advanced::graphics::text::Editor, Font = iced::Font>,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &Limits,
    ) -> layout::Node {
        let mut internal = self.content.0.borrow_mut();
        let state = tree.state.downcast_mut::<State<Highlighter>>();

        if state.highlighter_format_address != self.highlighter_format as usize {
            state.highlighter.borrow_mut().change_line(0);
            state.highlighter_format_address = self.highlighter_format as usize;
        }

        if state.highlighter_settings != self.highlighter_settings {
            state
                .highlighter
                .borrow_mut()
                .update(&self.highlighter_settings);

            state.highlighter_settings = self.highlighter_settings.clone();
        }

        let limits = limits.height(self.height);

        internal.editor.update(
            limits.shrink(self.padding).max(),
            self.font.unwrap_or_else(|| renderer.default_font()),
            self.text_size.unwrap_or_else(|| renderer.default_size()),
            self.line_height,
            state.highlighter.borrow_mut().deref_mut(),
        );

        match self.height {
            Length::Fill | Length::FillPortion(_) | Length::Fixed(_) => {
                layout::Node::new(limits.max())
            }
            Length::Shrink => {
                let min_bounds = internal.editor.min_bounds();
                layout::Node::new(
                    limits
                        .height(min_bounds.height)
                        .max()
                        .expand(Size::new(0.0, self.padding.vertical())),
                )
            }
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        let mut internal = self.content.0.borrow_mut();
        let state = tree.state.downcast_ref::<State<Highlighter>>();

        //internal.editor.highlight(
        //    self.font.unwrap_or_else(|| renderer.default_font()),
        //    state.highlighter.borrow_mut().deref_mut(),
        //    |highlight| (self.highlighter_format)(highlight, theme),
        //);

        let is_disabled = self.on_edit.is_none();
        let is_mouse_over = cursor.is_over(bounds);

        let appearance = if is_disabled {
            theme.disabled(&self.style)
        } else if state.is_focused {
            theme.focused(&self.style)
        } else if is_mouse_over {
            theme.hovered(&self.style)
        } else {
            theme.active(&self.style)
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: appearance.border,
                ..renderer::Quad::default()
            },
            appearance.background,
        );

        // Line numbers
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y,
                    width: self.line_numbers_width,
                    height: bounds.height,
                },
                border: appearance.border,
                ..renderer::Quad::default()
            },
            appearance.background,
        );

        let line_numbers_bounds = Rectangle {
            x: bounds.x + self.line_numbers_width - self.padding.left,
            y: bounds.y + self.padding.top,
            width: self.line_numbers_width - self.padding.horizontal(),
            height: bounds.height - self.padding.vertical(),
        };

        // Different because text alignment Horizontal::Right flips the bounds for drawing
        let line_numbers_clip_bounds = Rectangle {
            x: bounds.x + self.padding.left,
            ..line_numbers_bounds
        };

        let line_height_f32: f32 = self
            .line_height
            .to_absolute(self.text_size.unwrap_or_else(|| renderer.default_size()))
            .into();

        for (i, ln_str) in Self::line_numbers_str(&internal.editor).iter().enumerate() {
            let ln_bounds = Rectangle {
                y: line_numbers_bounds.y + i as f32 * line_height_f32,
                height: line_height_f32,
                ..line_numbers_bounds
            };
            let ln_text = Text {
                content: ln_str,
                bounds: ln_bounds.size(),
                size: self.text_size.unwrap_or_else(|| renderer.default_size()),
                line_height: self.line_height,
                font: self.font.unwrap_or_else(|| renderer.default_font()),
                horizontal_alignment: Horizontal::Right,
                vertical_alignment: Vertical::Top,
                shaping: Default::default(),
            };
            renderer.fill_text(
                ln_text,
                ln_bounds.position(),
                style.text_color,
                line_numbers_clip_bounds,
            )
        }

        let editor_bounds = self.editor_bounds(layout);

        renderer.fill_editor(
            &internal.editor,
            editor_bounds.position() + Vector::new(self.padding.left, self.padding.top),
            style.text_color,
            *viewport,
        );

        let translation = Vector::new(
            editor_bounds.x + self.padding.left,
            editor_bounds.y + self.padding.top,
        );

        if state.is_focused {
            match internal.editor.cursor() {
                Cursor::Caret(position) => {
                    let position = position + translation;

                    if bounds.contains(position) {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: position.x,
                                    y: position.y,
                                    width: 1.0,
                                    height: self
                                        .line_height
                                        .to_absolute(
                                            self.text_size
                                                .unwrap_or_else(|| renderer.default_size()),
                                        )
                                        .into(),
                                },
                                ..renderer::Quad::default()
                            },
                            theme.value_color(&self.style),
                        );
                    }
                }
                Cursor::Selection(ranges) => {
                    for range in ranges
                        .into_iter()
                        .filter_map(|range| bounds.intersection(&(range + translation)))
                    {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: range,
                                ..renderer::Quad::default()
                            },
                            theme.selection_color(&self.style),
                        );
                    }
                }
            }
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State<Highlighter>>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State {
            is_focused: false,
            last_click: None,
            drag_click: None,
            partial_scroll: 0.0,
            highlighter: RefCell::new(Highlighter::new(&self.highlighter_settings)),
            highlighter_settings: self.highlighter_settings.clone(),
            highlighter_format_address: self.highlighter_format as usize,
        })
    }

    fn on_event(
        &mut self,
        tree: &mut widget::Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let Some(on_edit) = self.on_edit.as_ref() else {
            return event::Status::Ignored;
        };

        let state = tree.state.downcast_mut::<State<Highlighter>>();

        let Some(update) = Update::from_event(
            event,
            state,
            self.editor_bounds(layout),
            self.padding,
            cursor,
        ) else {
            return event::Status::Ignored;
        };

        match update {
            Update::Click(click) => {
                let action = match click.kind() {
                    mouse::click::Kind::Single => Action::Click(click.position()),
                    mouse::click::Kind::Double => Action::SelectWord,
                    mouse::click::Kind::Triple => Action::SelectLine,
                };

                state.is_focused = true;
                state.last_click = Some(click);
                state.drag_click = Some(click.kind());

                shell.publish(on_edit(action));
            }
            Update::Scroll(lines) => {
                let lines = lines + state.partial_scroll;
                state.partial_scroll = lines.fract();
                shell.publish(on_edit(Action::Scroll {
                    lines: lines as i32,
                }));
            }
            Update::Unfocus => {
                state.is_focused = false;
                state.drag_click = None;
            }
            Update::Release => {
                state.drag_click = None;
            }
            Update::Action(action) => {
                shell.publish(on_edit(action));
            }
            Update::Copy => {
                if let Some(selection) = self.content.selection() {
                    clipboard.write(clipboard::Kind::Standard, selection);
                }
            }
        }

        event::Status::Captured
    }

    fn mouse_interaction(
        &self,
        _state: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let is_disabled = self.on_edit.is_none();

        if cursor.is_over(layout.bounds()) {
            if is_disabled {
                mouse::Interaction::NotAllowed
            } else {
                mouse::Interaction::Text
            }
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Highlighter, Message, Theme, Renderer>
    From<LogViewer<'a, Highlighter, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Highlighter: text::Highlighter,
    Message: 'a,
    Theme: StyleSheet + 'a,
    Renderer: text::Renderer<Editor = iced::advanced::graphics::text::Editor, Font = iced::Font>,
{
    fn from(log_viewer: LogViewer<'a, Highlighter, Message, Theme, Renderer>) -> Self {
        Self::new(log_viewer)
    }
}

enum Update {
    Click(mouse::Click),
    Scroll(f32),
    Unfocus,
    Release,
    Action(Action),
    Copy,
}

impl Update {
    fn from_event<H: Highlighter>(
        event: Event,
        state: &State<H>,
        bounds: Rectangle,
        padding: Padding,
        cursor: mouse::Cursor,
    ) -> Option<Self> {
        let action = |action| Some(Update::Action(action));

        match event {
            Event::Mouse(event) => match event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if let Some(cursor_position) = cursor.position_in(bounds) {
                        let cursor_position =
                            cursor_position - Vector::new(padding.top, padding.left);

                        let click = mouse::Click::new(cursor_position, state.last_click);

                        Some(Update::Click(click))
                    } else if state.is_focused {
                        Some(Update::Unfocus)
                    } else {
                        None
                    }
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => Some(Update::Release),
                mouse::Event::CursorMoved { .. } => match state.drag_click {
                    Some(mouse::click::Kind::Single) => {
                        let cursor_position =
                            cursor.position_in(bounds)? - Vector::new(padding.top, padding.left);

                        action(Action::Drag(cursor_position))
                    }
                    _ => None,
                },
                mouse::Event::WheelScrolled { delta } if cursor.is_over(bounds) => {
                    Some(Update::Scroll(match delta {
                        mouse::ScrollDelta::Lines { y, .. } => {
                            if y.abs() > 0.0 {
                                y.signum() * -(y.abs() * 4.0).max(1.0)
                            } else {
                                0.0
                            }
                        }
                        mouse::ScrollDelta::Pixels { y, .. } => -y / 4.0,
                    }))
                }
                _ => None,
            },
            Event::Keyboard(event) => match event {
                keyboard::Event::KeyPressed { key, modifiers, .. } if state.is_focused => {
                    match key.as_ref() {
                        keyboard::Key::Character("c") if modifiers.command() => {
                            return Some(Self::Copy);
                        }
                        _ => {}
                    }

                    if let keyboard::Key::Named(named_key) = key.as_ref() {
                        if let Some(motion) = motion(named_key) {
                            let motion = if platform::is_jump_modifier_pressed(modifiers) {
                                motion.widen()
                            } else {
                                motion
                            };

                            return action(if modifiers.shift() {
                                Action::Select(motion)
                            } else {
                                Action::Move(motion)
                            });
                        }
                    }

                    None
                }
                _ => None,
            },
            _ => None,
        }
    }
}

fn motion(key: key::Named) -> Option<Motion> {
    match key {
        key::Named::ArrowLeft => Some(Motion::Left),
        key::Named::ArrowRight => Some(Motion::Right),
        key::Named::ArrowUp => Some(Motion::Up),
        key::Named::Home => Some(Motion::Home),
        key::Named::End => Some(Motion::End),
        key::Named::PageUp => Some(Motion::PageUp),
        key::Named::PageDown => Some(Motion::PageDown),
        _ => None,
    }
}

mod platform {
    use iced::keyboard;

    pub fn is_jump_modifier_pressed(modifiers: keyboard::Modifiers) -> bool {
        if cfg!(target_os = "macos") {
            modifiers.alt()
        } else {
            modifiers.control()
        }
    }
}
