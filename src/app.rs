use ratatui::widgets::TableState;
use serde::Deserialize;
use std::{fs, io};

#[derive(Deserialize, Clone)]
pub struct Shortcut {
    pub source: String,
    pub trigger: String,
    pub keys: Vec<String>,
    pub action: String,
    pub desc: String,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Filter {
    All,
    Skhd,
    Karabiner,
    Xcode,
    System,
}

impl Filter {
    pub fn as_str(&self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Skhd => "sk",
            Filter::Karabiner => "ke",
            Filter::Xcode => "xcode",
            Filter::System => "sy",
        }
    }
}

pub struct App {
    pub state: TableState,
    pub items: Vec<Shortcut>,
    pub filter: Filter,
    pub search_query: String,
    pub is_searching: bool,
    pub show_help: bool,
    pub config_path: Option<String>,
    pub bulk_highlight: bool,
}

impl App {
    pub fn new(custom_path: Option<String>) -> Result<App, io::Error> {
        let path = if let Some(ref p) = custom_path {
            p.clone()
        } else {
            let home = std::env::var("HOME").map_err(|_| {
                io::Error::new(io::ErrorKind::NotFound, "HOME environment variable not set")
            })?;
            format!("{}/.config/karabiner/shortcuts.json", home)
        };

        let content = fs::read_to_string(&path)?;
        let items: Vec<Shortcut> = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut app = App {
            state: TableState::default(),
            items,
            filter: Filter::All,
            search_query: String::new(),
            is_searching: false,
            show_help: false,
            config_path: Some(path),
            bulk_highlight: false,
        };
        app.state.select(Some(0));
        Ok(app)
    }

    pub fn reload(&mut self) -> Result<(), io::Error> {
        if let Some(ref path) = self.config_path {
            let content = fs::read_to_string(path)?;
            let items: Vec<Shortcut> = serde_json::from_str(&content)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            self.items = items;
            if self.state.selected().is_none() && !self.items.is_empty() {
                self.state.select(Some(0));
            }
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Config path not known",
            ))
        }
    }

    pub fn sort_shortcuts(&mut self) {
        self.items.sort_by(|a, b| a.desc.cmp(&b.desc));
    }

    pub fn next(&mut self) {
        let filtered_len = self.filtered_items().len();
        if filtered_len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= filtered_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let filtered_len = self.filtered_items().len();
        if filtered_len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    filtered_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn filtered_items(&self) -> Vec<&Shortcut> {
        let query = self.search_query.to_lowercase();
        let filter_str = self.filter.as_str();

        self.items
            .iter()
            .filter(|i| {
                // Category filter
                if self.filter != Filter::All && !i.source.to_lowercase().contains(filter_str) {
                    return false;
                }

                // Text search filter
                if query.is_empty() {
                    return true;
                }

                i.action.to_lowercase().contains(&query)
                    || i.desc.to_lowercase().contains(&query)
                    || i.trigger.to_lowercase().contains(&query)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_shortcut(source: &str) -> Shortcut {
        Shortcut {
            source: source.to_string(),
            trigger: "trigger".into(),
            keys: vec!["key".into()],
            action: "action".into(),
            desc: "desc".into(),
        }
    }

    #[test]
    fn test_pagination() {
        let mut app = App {
            state: TableState::default(),
            items: vec![mock_shortcut("a"), mock_shortcut("b"), mock_shortcut("c")],
            filter: Filter::All,
            search_query: String::new(),
            is_searching: false,
            show_help: false,
            config_path: None,
            bulk_highlight: false,
        };
        app.state.select(Some(0));

        app.next();
        assert_eq!(app.state.selected(), Some(1));
        app.next();
        assert_eq!(app.state.selected(), Some(2));
        app.next();
        assert_eq!(app.state.selected(), Some(0)); // Wrap

        app.previous();
        assert_eq!(app.state.selected(), Some(2)); // Wrap back
    }

    #[test]
    fn test_filtering() {
        let app = App {
            state: TableState::default(),
            items: vec![
                mock_shortcut("skhd"),
                mock_shortcut("karabiner"),
                mock_shortcut("xcode"),
            ],
            filter: Filter::Skhd,
            search_query: String::new(),
            is_searching: false,
            show_help: false,
            config_path: None,
            bulk_highlight: false,
        };

        let filtered = app.filtered_items();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source, "skhd");
    }
}
