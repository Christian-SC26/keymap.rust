use ratatui::widgets::TableState;
use serde::Deserialize;
use std::{fs, io};

#[derive(Deserialize, Clone)]
pub struct Shortcut {
    pub source: String,
    #[serde(default)]
    pub rules: String,
    pub keys: Vec<String>,
    pub action: String,
    pub desc: String,

    #[serde(skip)]
    pub search_text: String,
}

#[derive(Deserialize, Clone)]
pub struct KeyDef {
    pub display: String,
    pub id: String,
    pub width: usize,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Filter {
    All,
    Skhd,
    Karabiner,
    System,
}

impl Filter {
    pub fn as_str(&self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Skhd => "sk",
            Filter::Karabiner => "ke",
            Filter::System => "sy",
        }
    }
}

pub struct App {
    pub state: TableState,
    pub items: Vec<Shortcut>,
    pub filter: Filter,
    pub search_query: String,
    pub app_filter_query: String,
    pub key_filter: Option<char>,
    pub is_searching: bool,
    pub is_filtering_app: bool,
    pub is_filtering_key: bool,
    pub show_help: bool,
    pub show_overview: bool,
    pub config_path: Option<String>,
    pub bulk_highlight: bool,
    pub keyboard_layout: Vec<Vec<KeyDef>>,
}

impl App {
    pub fn new(custom_path: Option<String>) -> Result<App, io::Error> {
        let home = std::env::var("HOME").map_err(|_| {
            io::Error::new(io::ErrorKind::NotFound, "HOME environment variable not set")
        })?;
        
        let path = if let Some(ref p) = custom_path {
            p.clone()
        } else {
            "src/shortcuts.json".to_string()
        };

        let content = fs::read_to_string(&path)?;
        let mut items: Vec<Shortcut> = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        #[derive(Deserialize)]
        struct SystemShortcut {
            mods: Vec<String>,
            key: String,
            desc: String,
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let sys_paths = [
            "system_shortcuts.json".to_string(),
            "src/system_shortcuts.json".to_string(),
            format!("{}/.config/karabiner/system_shortcuts.json", home),
        ];

        for sys_path in sys_paths {
            if let Ok(c) = fs::read_to_string(&sys_path) {
                if let Ok(sys_items) = serde_json::from_str::<Vec<SystemShortcut>>(&c) {
                    for sys_item in sys_items {
                        let mut keys = sys_item.mods.clone();
                        keys.push(sys_item.key.clone());
                        items.push(Shortcut {
                            source: "system sy".to_string(),
                            rules: String::new(),
                            keys,
                            action: "-".to_string(),
                            desc: sys_item.desc.clone(),
                            search_text: String::new(),
                        });
                    }
                    break;
                }
            }
        }

        for item in &mut items {
            item.search_text = format!("{} {} {} {}", item.action, item.desc, item.keys.join(" "), item.rules).to_lowercase();
        }

        // Загрузка динамической раскладки клавиатуры
        let layout_paths = [
            "keyboard_layout.json".to_string(),
            "src/keyboard_layout.json".to_string(),
            format!("{}/.config/karabiner/keyboard_layout.json", home),
        ];

        let mut keyboard_layout: Vec<Vec<KeyDef>> = Vec::new();
        for layout_path in layout_paths {
            if let Ok(c) = fs::read_to_string(&layout_path) {
                if let Ok(parsed) = serde_json::from_str(&c) {
                    keyboard_layout = parsed;
                    break;
                }
            }
        }
        
        if keyboard_layout.is_empty() {
            eprintln!("[Warn] Failed to load keyboard_layout.json");
        }

        let mut app = App {
            state: TableState::default(),
            items,
            filter: Filter::All,
            search_query: String::new(),
            app_filter_query: String::new(),
            key_filter: None,
            is_searching: false,
            is_filtering_app: false,
            is_filtering_key: false,
            show_help: false,
            show_overview: false,
            config_path: Some(path),
            bulk_highlight: false,
            keyboard_layout,
        };
        app.state.select(Some(0));
        Ok(app)
    }

    pub fn reload(&mut self) -> Result<(), io::Error> {
        if let Some(ref path) = self.config_path {
            let content = fs::read_to_string(&path)?;
            let mut items: Vec<Shortcut> = serde_json::from_str(&content)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            #[derive(Deserialize)]
            struct SystemShortcut {
                mods: Vec<String>,
                key: String,
                desc: String,
            }

            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let sys_paths = [
                "system_shortcuts.json".to_string(),
                "src/system_shortcuts.json".to_string(),
                format!("{}/.config/karabiner/system_shortcuts.json", home),
            ];

            for sys_path in sys_paths {
                if let Ok(c) = fs::read_to_string(&sys_path) {
                    if let Ok(sys_items) = serde_json::from_str::<Vec<SystemShortcut>>(&c) {
                        for sys_item in sys_items {
                            let mut keys = sys_item.mods.clone();
                            keys.push(sys_item.key.clone());
                            items.push(Shortcut {
                                source: "system sy".to_string(),
                                rules: String::new(),
                                keys,
                                action: "-".to_string(),
                                desc: sys_item.desc.clone(),
                                search_text: String::new(),
                            });
                        }
                        break;
                    }
                }
            }

            for item in &mut items {
                item.search_text = format!("{} {} {} {}", item.action, item.desc, item.keys.join(" "), item.rules).to_lowercase();
            }

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

    pub fn jump_down(&mut self) {
        let filtered_len = self.filtered_items().len();
        if filtered_len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 20).min(filtered_len - 1),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn jump_up(&mut self) {
        let filtered_len = self.filtered_items().len();
        if filtered_len == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(20),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn filtered_items(&self) -> Vec<&Shortcut> {
        let query = self.search_query.to_lowercase();
        let app_q = self.app_filter_query.to_lowercase();
        let filter_str = self.filter.as_str();

        self.items
            .iter()
            .filter(|i| {
                // Core Source Filter
                if self.filter != Filter::All && !i.source.contains(filter_str) {
                    return false;
                }

                // Sub-filter for App (only if in Karabiner filter)
                if self.filter == Filter::Karabiner && !app_q.is_empty() {
                    let enable_tag = format!("{}_e", app_q);
                    let disable_tag = format!("{}_d", app_q);
                    if !i.source.contains(&enable_tag) && !i.source.contains(&disable_tag) {
                        return false;
                    }
                }
                
                // Key Filter (Space mode)
                if self.is_filtering_key {
                    if let Some(target_char) = self.key_filter {
                        let target_str = target_char.to_string().to_lowercase();
                        if !i.keys.iter().any(|k| k.to_lowercase() == target_str) {
                            return false;
                        }
                    }
                }

                // Text search filter
                if query.is_empty() {
                    return true;
                }

                i.search_text.contains(&query)
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
            rules: String::new(),
            keys: vec!["key".into()],
            action: "action".into(),
            desc: "desc".into(),
            search_text: "action desc key ".into(),
        }
    }

    #[test]
    fn test_pagination() {
        let mut app = App {
            state: TableState::default(),
            items: vec![mock_shortcut("a"), mock_shortcut("b"), mock_shortcut("c")],
            filter: Filter::All,
            search_query: String::new(),
            app_filter_query: String::new(),
            key_filter: None,
            is_searching: false,
            is_filtering_app: false,
            is_filtering_key: false,
            show_help: false,
            show_overview: false,
            config_path: None,
            bulk_highlight: false,
            keyboard_layout: vec![],
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
                mock_shortcut("sk"),
                mock_shortcut("ke"),
                mock_shortcut("sy"),
            ],
            filter: Filter::Skhd,
            search_query: String::new(),
            app_filter_query: String::new(),
            key_filter: None,
            is_searching: false,
            is_filtering_app: false,
            is_filtering_key: false,
            show_help: false,
            show_overview: false,
            config_path: None,
            bulk_highlight: false,
            keyboard_layout: vec![],
        };

        let filtered = app.filtered_items();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source, "sk");
    }
}
