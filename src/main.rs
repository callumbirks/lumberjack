use enum_iterator::all;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::{Arc, Weak};

use crate::data::repl::Repl;
use crate::data::{
    DBEvent, LogEventType, LogLevel, LogLine, LogObject, LogObjectType, PullerEvent, PusherEvent,
    ReplEvent,
};
use iced::futures::TryFutureExt;
use iced::highlighter::{self};
use iced::widget::shader::wgpu::naga::FastHashMap;
use iced::widget::{button, container, pick_list, scrollable, text, text_input, Column, Row};
use iced::{keyboard, Command, Element, Length, Subscription};
use iced::{Application, Font, Settings};

use crate::error::{LumberjackError, Result};
use crate::parse::puller::Puller;
use crate::parse::pusher::Pusher;
use crate::parse::{db::DB, LogHolder, LogParser};
use crate::util::{open_folder, ContainsWithCase, Truncate};
use crate::widget::log_table;

mod data;
mod error;
mod lumberjack;
mod parse;
mod util;
mod widget;

fn main() -> iced::Result {
    env_logger::init();
    App::run(Settings {
        ..Settings::default()
    })
}

#[derive(Debug, Clone)]
enum FilterType {
    Level(LogLevel),
    ObjectType(LogObjectType),
    Object(Weak<LogObject>),
    Event(LogEventType),
    Message(String),
}

#[derive(Debug, Clone)]
enum Message {
    Initialise,
    Initialised(Result<LogHolder>),
    SelectFilter(FilterType),
    SetFilter(Filter),
    DoneFilter(log_table::Content<Arc<LogLine>>),
    RowClicked(u64, Arc<LogLine>),
    Scrolled(scrollable::Viewport),
}

#[derive(Default, Debug, Clone)]
pub struct Filter {
    level: Option<LogLevel>,
    object_type: Option<LogObjectType>,
    object: Option<Arc<LogObject>>,
    event: Option<LogEventType>,
    message: Box<str>,
}

struct App {
    log_lines: Vec<Arc<LogLine>>,
    objects: FastHashMap<LogObjectType, Vec<Arc<LogObject>>>,
    content: log_table::Content<Arc<LogLine>>,
    selected_line: Option<Arc<LogLine>>,
    filter: Filter,
    last_error: Option<LumberjackError>,
    is_loading: bool,
    theme: highlighter::Theme,
    font: Option<Font>,
}

impl Application for App {
    type Executor = iced::executor::Default;

    type Message = Message;

    type Theme = iced::Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                log_lines: vec![],
                objects: FastHashMap::default(),
                content: log_table::Content::new_empty(),
                selected_line: None,
                filter: Filter::default(),
                last_error: None,
                is_loading: false,
                theme: highlighter::Theme::SolarizedDark,
                font: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!("Lumberjack {} 🪵🪓", env!("CARGO_PKG_VERSION"))
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        log::trace!("Received message: {:?}", message);
        match message {
            Message::Initialise => {
                self.is_loading = true;
                Command::perform(initialise(), Message::Initialised)
            }
            Message::Initialised(log_holder) => {
                self.is_loading = false;
                match log_holder {
                    Ok(log_holder) => {
                        self.log_lines = log_holder.log_lines;
                        self.objects = log_holder.objects;
                    }
                    Err(err) => self.last_error = Some(err),
                }
                Command::perform(
                    content_with_filter(self.log_lines.clone(), self.filter.clone()),
                    Message::DoneFilter,
                )
            }
            Message::SetFilter(filter) => {
                self.filter = filter;
                self.is_loading = true;
                Command::perform(
                    content_with_filter(self.log_lines.clone(), self.filter.clone()),
                    Message::DoneFilter,
                )
            }
            Message::DoneFilter(content) => {
                self.content = content;
                Command::none()
            }
            Message::RowClicked(i, line) => {
                self.selected_line = Some(line);
                self.content.focus_line(i)
            }
            Message::SelectFilter(filter) => {
                match filter {
                    FilterType::Level(level) => {
                        self.filter.level = match level {
                            LogLevel::None => None,
                            other => Some(other),
                        }
                    }
                    FilterType::ObjectType(object_type) => {
                        self.filter.object_type = match object_type {
                            LogObjectType::None => None,
                            other => Some(other),
                        };
                        self.filter.object = None;
                    }
                    FilterType::Object(object) => self.filter.object = object.upgrade(),
                    FilterType::Event(event) => {
                        self.filter.event = match event {
                            LogEventType::None => None,
                            other => Some(other),
                        }
                    }
                    FilterType::Message(filter) => {
                        self.filter.message = filter.into_boxed_str();
                    }
                }
                Command::perform(
                    content_with_filter(self.log_lines.clone(), self.filter.clone()),
                    Message::DoneFilter,
                )
            }
            Message::Scrolled(_) => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        if self.log_lines.is_empty() {
            return container(button("Load CBL logs").on_press(Message::Initialise))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_y()
                .center_x()
                .into();
        }

        let filters = Row::with_children([
            pick_list(
                all::<LogLevel>().collect::<Box<_>>(),
                self.filter.level,
                |level| Message::SelectFilter(FilterType::Level(level)),
            )
            .placeholder("Level")
            .into(),
            pick_list(
                all::<LogObjectType>().collect::<Box<_>>(),
                self.filter.object_type,
                |obj_type| Message::SelectFilter(FilterType::ObjectType(obj_type)),
            )
            .placeholder("Object Type")
            .into(),
            pick_list(self.objects_of_type(), self.filter.object.clone(), |obj| {
                Message::SelectFilter(FilterType::Object(Arc::downgrade(&obj)))
            })
            .placeholder("Object")
            .into(),
            pick_list(self.events_of_type(), self.filter.event, |event| {
                Message::SelectFilter(FilterType::Event(event))
            })
            .placeholder("Event")
            .into(),
            text_input("Filter Message", &self.filter.message)
                .on_input(|input| Message::SelectFilter(FilterType::Message(input)))
                .into(),
        ])
        .spacing(10.0)
        .padding(10.0)
        .into();

        let table = log_table::LogTable::with_content(&self.content, Message::Scrolled)
            .column_widths([
                Length::Fixed(200.0),
                Length::Fixed(120.0),
                Length::Fixed(180.0),
                Length::Fill,
            ])
            .on_click_row(Message::RowClicked)
            .font(Font::MONOSPACE);

        Column::with_children([
            filters,
            table.height(Length::FillPortion(3)).into(),
            Row::with_children([
                Column::with_children([
                    text("Message").into(),
                    if let Some(line) = &self.selected_line {
                        text(&line.message).into()
                    } else {
                        text("").into()
                    },
                ])
                .width(Length::Fill)
                .into(),
                Column::with_children([
                    text("Object").into(),
                    if let Some(object) =
                        self.selected_line.as_ref().and_then(|l| l.object.as_ref())
                    {
                        text(object.details()).into()
                    } else {
                        text("").into()
                    },
                ])
                .width(Length::Fill)
                .into(),
            ])
            .padding(5.0)
            .width(Length::Fill)
            .height(Length::FillPortion(1))
            .into(),
        ])
        .into()
    }

    fn theme(&self) -> Self::Theme {
        iced::Theme::KanagawaDragon
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::on_key_press(|key, modifiers| match key.as_ref() {
            _ => None,
        })
    }
}

impl App {
    pub fn objects_of_type(&self) -> Vec<Arc<LogObject>> {
        static EMPTY: Vec<Arc<LogObject>> = vec![];
        if let Some(selected_type) = self.filter.object_type {
            self.objects[&selected_type].clone()
        } else {
            EMPTY.clone()
        }
    }

    pub fn events_of_type(&self) -> Vec<LogEventType> {
        let mut defaults = vec![
            LogEventType::None,
            LogEventType::Created,
            LogEventType::Destroyed,
        ];
        let Some(selected_type) = self.filter.object_type else {
            return defaults;
        };
        match selected_type {
            LogObjectType::None => unreachable!(),
            LogObjectType::DB => {
                defaults.extend(all::<DBEvent>().map(LogEventType::DB));
                defaults
            }
            LogObjectType::Repl => {
                defaults.extend(all::<ReplEvent>().map(LogEventType::Repl));
                defaults
            }
            LogObjectType::Puller => {
                defaults.extend(all::<PullerEvent>().map(LogEventType::Puller));
                defaults
            }
            LogObjectType::Pusher => {
                defaults.extend(all::<PusherEvent>().map(LogEventType::Pusher));
                defaults
            }
            LogObjectType::Query => unreachable!(),
            LogObjectType::QueryEnum => unreachable!(),
        }
    }
}

pub async fn initialise() -> Result<LogHolder> {
    open_folder()
        .and_then(LogParser::parse::<Repl>)
        .and_then(LogParser::parse::<DB>)
        .and_then(LogParser::parse::<Puller>)
        .and_then(LogParser::parse::<Pusher>)
        .await
        .and_then(LogParser::finish)
}

pub async fn content_with_filter(
    lines: Vec<Arc<LogLine>>,
    filter: Filter,
) -> log_table::Content<Arc<LogLine>> {
    let lines = filter_lines(lines, filter).await;
    log_table::Content::new_with(
        ["Timestamp", "Object", "Event", "Message"],
        &lines,
        |line| {
            let mut message = line.message.truncate(48).to_string();
            if line.message.len() > message.len() {
                message.push_str("...");
            }
            log_table::Row::new_with(
                line,
                [
                    line.timestamp.to_string(),
                    line.object
                        .as_ref()
                        .map_or_else(|| "".to_string(), |o| o.name()),
                    line.event
                        .map_or_else(|| "".to_string(), |event| event.to_string()),
                    message,
                ],
            )
        },
    )
}

pub async fn filter_lines(lines: Vec<Arc<LogLine>>, filter: Filter) -> Vec<Arc<LogLine>> {
    tokio::spawn(async move {
        let mut predicate = filter.into_predicate();
        lines
            .into_iter()
            .filter(|line| predicate(line.as_ref()))
            .collect()
    })
    .await
    .unwrap_or_default()
}

impl Filter {
    pub fn into_predicate(self) -> impl FnMut(&LogLine) -> bool {
        move |log_line: &LogLine| {
            (self.level.is_some_and(|level| log_line.file.level == level) || self.level.is_none())
                && (self.object_type.is_some_and(|obj_type| {
                    log_line
                        .object
                        .as_ref()
                        .is_some_and(|obj| obj.object_type == obj_type)
                }) || self.object_type.is_none())
                && (self.object == log_line.object || self.object.is_none())
                // Comment self.event.is_none() to see lines with not-parsed events
                && (self.event == log_line.event || self.event.is_none())
                && (self.message.is_empty()
                    || log_line.message.contains_with_case(self.message.as_ref()))
        }
    }
}
