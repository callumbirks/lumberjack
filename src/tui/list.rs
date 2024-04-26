use ratatui::prelude::Stylize;
use ratatui::widgets::{ListItem, ListState};
use crate::event::repl::Repl;
use crate::lumberjack::LogLine;
use crate::{ALT_ROW_COLOR, NORMAL_ROW_COLOR};

#[derive(Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
    pub last_selected: Option<usize>,
}

impl<T> Default for StatefulList<T> {
    fn default() -> Self {
        StatefulList {
            state: ListState::default(),
            items: vec![],
            last_selected: None,
        }
    }
}

impl<T> StatefulList<T>
    where T: AsListItem
{
    pub fn with_items(list: impl IntoIterator<Item=T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: list.into_iter().collect(),
            last_selected: None,
        }
    }

    pub fn set_items(&mut self, list: impl IntoIterator<Item=T>) {
        self.items.clear();
        for item in list {
            self.items.push(item);
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 { 0 } else { i + 1 }
            }
            None => self.last_selected.unwrap_or(0),
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 { self.items.len() - 1 } else { i - 1 }
            }
            None => self.last_selected.unwrap_or(0),
        };
        self.state.select(Some(i))
    }

    pub fn unselect(&mut self) {
        let offset = self.state.offset();
        self.last_selected = self.state.selected();
        self.state.select(None);
        *self.state.offset_mut() = offset;
    }
}

pub trait AsListItem {
    fn as_list_item(&self, index: usize) -> ListItem;
}

macro_rules! enum_listitem {
    (pub enum $enum_name:ident {
        $($variant:ident => $str:expr),+
    }) => {
        #[derive(Copy, Clone)]
        pub enum $enum_name {
            $($variant),+
        }

        impl AsListItem for $enum_name {
            fn as_list_item(&self, index: usize) -> ListItem {
                let color = if index % 2 == 0 {
                    ALT_ROW_COLOR
                } else {
                    NORMAL_ROW_COLOR
                };
                ListItem::new(match self {
                    $($enum_name::$variant => $str),+
                }).fg(color)
            }
        }

        impl $enum_name {
            pub fn all() -> Vec<$enum_name> {
                vec![
                    $($enum_name::$variant),+
                ]
            }
        }
    }
}

enum_listitem!(pub enum MainMenu {
    ObjectList => "Object Browser"
});

enum_listitem!(pub enum ObjectList {
    ReplObjects => "Repl Objects"
});

impl AsListItem for Repl {
    fn as_list_item(&self, index: usize) -> ListItem {
        let color = if index % 2 == 0 {
            ALT_ROW_COLOR
        } else {
            NORMAL_ROW_COLOR
        };
        ListItem::new(format!("Repl#{}", self.id)).fg(color)
    }
}

impl AsListItem for LogLine {
    fn as_list_item(&self, index: usize) -> ListItem {
        let color = if index % 2 == 0 {
            ALT_ROW_COLOR
        } else {
            NORMAL_ROW_COLOR
        };
        ListItem::new(self.read().unwrap_or("ERROR READING LINE".to_string()))
    }
}