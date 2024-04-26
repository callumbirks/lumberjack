use ratatui::prelude::{Buffer, Rect, Widget};
use ratatui::widgets::{ListItem, ListState, Paragraph};
use crate::event::EventGroup;
use crate::event::repl::Repl;
use crate::lumberjack::{LogLine, Lumberjack};
use crate::tui::{StatefulList};
use crate::tui::list::{MainMenu, ObjectList, AsListItem};

pub struct State {
    current_menu: Menu,
    main_menu: StatefulList<MainMenu>,
    object_list: StatefulList<ObjectList>,
    repl_items: StatefulList<Repl>,
    log_lines: StatefulList<LogLine>,
}

pub enum Menu {
    MainMenu,
    ObjectList,
    ReplList,
    LogView,
}

impl State {
    pub fn new(lumberjack: &Lumberjack) -> Self {
        let repl_items = StatefulList::with_items(
            Repl::from_lumberjack(lumberjack)
                .expect("Failed to parse Repl objects"));

        let main_menu = StatefulList::with_items(MainMenu::all());

        let object_list = StatefulList::with_items(ObjectList::all());

        State {
            current_menu: Menu::MainMenu,
            main_menu,
            object_list,
            repl_items,
            log_lines: StatefulList::default(),
        }
    }

    pub fn back(&mut self) {
        match self.current_menu {
            Menu::MainMenu => {}
            Menu::ObjectList => self.current_menu = Menu::MainMenu,
            Menu::ReplList => self.current_menu = Menu::ObjectList,
            Menu::LogView => self.current_menu = Menu::ReplList,
        }
    }

    pub fn select(&mut self) {
        let Some(select_idx) = self.selected() else {
            return;
        };

        match self.current_menu {
            Menu::MainMenu => {
                match self.main_menu.items.get(select_idx) {
                    Some(MainMenu::ObjectList) => self.current_menu = Menu::ObjectList,
                    None => {}
                }
            }
            Menu::ObjectList => {
                match self.object_list.items.get(select_idx) {
                    Some(ObjectList::ReplObjects) => self.current_menu = Menu::ReplList,
                    None => {}
                }
            }
            Menu::ReplList => {
                match self.repl_items.items.get(select_idx) {
                    Some(repl) => {
                        self.log_lines.set_items(repl.lines.clone());
                        self.current_menu = Menu::LogView
                    }
                    None => {}
                }
            }
            Menu::LogView => {}
        }
    }

    pub fn title(&self) -> String {
        match self.current_menu {
            Menu::MainMenu => "Main Menu".to_string(),
            Menu::ObjectList => "Object List".to_string(),
            Menu::ReplList => "Repl Objects".to_string(),
            Menu::LogView => "Log View".to_string(),
        }
    }

    pub fn current_list(&mut self) -> Option<(&mut ListState, Vec<ListItem>)> {
        let index = self.selected()?;
        Some(match self.current_menu {
            Menu::MainMenu => {
                (&mut self.main_menu.state, self.main_menu.items.iter()
                    .map(|e| e.as_list_item(index)).collect())
            }
            Menu::ObjectList => {
                (&mut self.object_list.state, self.object_list.items.iter().map(|e| e.as_list_item(index)).collect())
            }
            Menu::ReplList => {
                (&mut self.repl_items.state, self.repl_items.items.iter().map(|e| e.as_list_item(index)).collect())
            }
            Menu::LogView => {
                (&mut self.log_lines.state, self.log_lines.items.iter().map(|e| e.as_list_item(index)).collect())
            }
        })
    }

    pub fn render_info(&self, area: Rect, buf: &mut Buffer) {
        let text = match self.current_menu {
            Menu::MainMenu => "".to_string(),
            Menu::ObjectList => "".to_string(),
            // Display the Repl config
            Menu::ReplList => {
                let selected = self.selected().and_then(|i| self.repl_items.items.get(i));
                if let Some(selected) = selected {
                    format!("{:?}", selected.config)
                } else {
                    "".to_string()
                }
            }
            Menu::LogView => {
                "".to_string()
            }
        };

        let paragraph = Paragraph::new(text);
        paragraph.render(area, buf);
    }

    pub fn up(&mut self) {
        match self.current_menu {
            Menu::MainMenu => self.main_menu.previous(),
            Menu::ObjectList => self.object_list.previous(),
            Menu::ReplList => self.repl_items.previous(),
            Menu::LogView => self.log_lines.previous(),
        }
    }

    pub fn down(&mut self) {
        match self.current_menu {
            Menu::MainMenu => self.main_menu.next(),
            Menu::ObjectList => self.object_list.next(),
            Menu::ReplList => self.repl_items.next(),
            Menu::LogView => self.log_lines.next(),
        }
    }

    fn selected(&self) -> Option<usize> {
        match self.current_menu {
            Menu::MainMenu => self.main_menu.state.selected(),
            Menu::ObjectList => self.object_list.state.selected(),
            Menu::ReplList => self.repl_items.state.selected(),
            Menu::LogView => self.log_lines.state.selected(),
        }
    }
}