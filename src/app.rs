use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};
use std::{fs, io};
use std::collections::HashSet;
use std::path::PathBuf;

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KeyDef {
    pub display: String,
    pub id: String,
    pub width: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KeyboardConfig {
    pub name: String,
    pub layout: Vec<Vec<KeyDef>>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EditMode {
    None,
    Visual,
    KeyInput,
    KeyboardNameInput,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EditField {
    KeyDisplay,
    KeyId,
    KeyboardName,
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
            Filter::Skhd => "skhd",
            Filter::Karabiner => "karabiner",
            Filter::System => "system",
        }
    }
}

pub struct SourceAnalysis {
    pub total_shortcuts: usize,
    pub top_modifier: String,
    pub config_path: String,
    pub last_modified: String,
    pub conflicts: Vec<(String, String, String)>, // (normalized_key, other_source, other_action)
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
    pub is_filtering_modifier: bool,
    pub active_modifiers: HashSet<String>,
    pub show_help: bool,
    pub show_overview: bool,
    pub config_path: Option<String>,
    pub bulk_highlight: bool,
    pub keyboards: Vec<KeyboardConfig>,
    pub filtered_indices: Vec<usize>,
    pub aliases: std::collections::HashMap<String, String>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub selected_keyboard_idx: usize,
    pub keyboard_dropdown_idx: usize,
    pub show_keyboard_dropdown: bool,
    pub edit_mode: EditMode,
    pub edit_selected_row: usize,
    pub edit_selected_col: usize,
    pub edit_input_buffer: String,
    pub edit_input_field: EditField,
}

impl App {
    fn load_system_shortcuts(home: &str) -> Vec<Shortcut> {
        #[derive(Deserialize)]
        struct SystemShortcut {
            mods: Vec<String>,
            key: String,
            desc: String,
        }

        let sys_paths = [
            "system_shortcuts.json".to_string(),
            "src/system_shortcuts.json".to_string(),
            format!("{}/.config/karabiner/system_shortcuts.json", home),
        ];

        let mut sys_shortcuts = Vec::new();
        for sys_path in sys_paths {
            if let Ok(c) = fs::read_to_string(&sys_path) {
                if let Ok(sys_items) = serde_json::from_str::<Vec<SystemShortcut>>(&c) {
                    for sys_item in sys_items {
                        let mut keys = sys_item.mods.clone();
                        keys.push(sys_item.key.clone());
                        sys_shortcuts.push(Shortcut {
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
        sys_shortcuts
    }

    fn load_app_aliases(home: &str) -> std::collections::HashMap<String, String> {
        let mut aliases = std::collections::HashMap::new();
        let alias_paths = [
            "app_aliases.json".to_string(),
            "src/app_aliases.json".to_string(),
            format!("{}/.config/karabiner/app_aliases.json", home),
        ];

        for path in &alias_paths {
            if let Ok(c) = fs::read_to_string(path) {
                if let Ok(parsed) = serde_json::from_str::<std::collections::HashMap<String, String>>(&c) {
                    aliases = parsed;
                    break;
                }
            }
        }
        aliases
    }

    fn keyboards_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(format!("{}/.config/karabiner/keyboards", home))
    }

    pub fn default_keychron_layout() -> KeyboardConfig {
        // Загрузка из keyboard_layout.json (основная раскладка Keychron)
        let layout_paths = [
            "keyboard_layout.json".to_string(),
            "src/keyboard_layout.json".to_string(),
            format!("{}/.config/karabiner/keyboard_layout.json", std::env::var("HOME").unwrap_or_default()),
        ];
        let mut layout: Vec<Vec<KeyDef>> = Vec::new();
        for path in layout_paths {
            if let Ok(c) = fs::read_to_string(&path) {
                if let Ok(parsed) = serde_json::from_str(&c) {
                    layout = parsed;
                    break;
                }
            }
        }
        KeyboardConfig {
            name: "Keychron K3D3".to_string(),
            layout,
        }
    }

    pub fn default_mbp_layout() -> KeyboardConfig {
        KeyboardConfig {
            name: "MBP Pro M3 Pro".to_string(),
            layout: vec![
                vec![
                    KeyDef { display: "esc".to_string(), id: "esc".to_string(), width: 9 },
                    KeyDef { display: "F1".to_string(), id: "f1".to_string(), width: 6 },
                    KeyDef { display: "F2".to_string(), id: "f2".to_string(), width: 6 },
                    KeyDef { display: "F3".to_string(), id: "f3".to_string(), width: 6 },
                    KeyDef { display: "F4".to_string(), id: "f4".to_string(), width: 6 },
                    KeyDef { display: "F5".to_string(), id: "f5".to_string(), width: 6 },
                    KeyDef { display: "F6".to_string(), id: "f6".to_string(), width: 6 },
                    KeyDef { display: "F7".to_string(), id: "f7".to_string(), width: 6 },
                    KeyDef { display: "F8".to_string(), id: "f8".to_string(), width: 6 },
                    KeyDef { display: "F9".to_string(), id: "f9".to_string(), width: 6 },
                    KeyDef { display: "F10".to_string(), id: "f10".to_string(), width: 6 },
                    KeyDef { display: "F11".to_string(), id: "f11".to_string(), width: 6 },
                    KeyDef { display: "F12".to_string(), id: "f12".to_string(), width: 6 },
                    KeyDef { display: "power".to_string(), id: "power".to_string(), width: 15 },
                ],
                vec![
                    KeyDef { display: "~".to_string(), id: "grave_accent_and_tilde".to_string(), width: 6 },
                    KeyDef { display: "1".to_string(), id: "1".to_string(), width: 6 },
                    KeyDef { display: "2".to_string(), id: "2".to_string(), width: 6 },
                    KeyDef { display: "3".to_string(), id: "3".to_string(), width: 6 },
                    KeyDef { display: "4".to_string(), id: "4".to_string(), width: 6 },
                    KeyDef { display: "5".to_string(), id: "5".to_string(), width: 6 },
                    KeyDef { display: "6".to_string(), id: "6".to_string(), width: 6 },
                    KeyDef { display: "7".to_string(), id: "7".to_string(), width: 6 },
                    KeyDef { display: "8".to_string(), id: "8".to_string(), width: 6 },
                    KeyDef { display: "9".to_string(), id: "9".to_string(), width: 6 },
                    KeyDef { display: "0".to_string(), id: "0".to_string(), width: 6 },
                    KeyDef { display: "-".to_string(), id: "hyphen".to_string(), width: 6 },
                    KeyDef { display: "=".to_string(), id: "equal_sign".to_string(), width: 6 },
                    KeyDef { display: "back".to_string(), id: "backspace".to_string(), width: 18 },
                ],
                vec![
                    KeyDef { display: "tab".to_string(), id: "tab".to_string(), width: 12 },
                    KeyDef { display: "q".to_string(), id: "q".to_string(), width: 6 },
                    KeyDef { display: "w".to_string(), id: "w".to_string(), width: 6 },
                    KeyDef { display: "e".to_string(), id: "e".to_string(), width: 6 },
                    KeyDef { display: "r".to_string(), id: "r".to_string(), width: 6 },
                    KeyDef { display: "t".to_string(), id: "t".to_string(), width: 6 },
                    KeyDef { display: "y".to_string(), id: "y".to_string(), width: 6 },
                    KeyDef { display: "u".to_string(), id: "u".to_string(), width: 6 },
                    KeyDef { display: "i".to_string(), id: "i".to_string(), width: 6 },
                    KeyDef { display: "o".to_string(), id: "o".to_string(), width: 6 },
                    KeyDef { display: "p".to_string(), id: "p".to_string(), width: 6 },
                    KeyDef { display: "[".to_string(), id: "open_bracket".to_string(), width: 6 },
                    KeyDef { display: "]".to_string(), id: "close_bracket".to_string(), width: 6 },
                    KeyDef { display: "\\".to_string(), id: "backslash".to_string(), width: 12 },
                ],
                vec![
                    KeyDef { display: "caps".to_string(), id: "caps".to_string(), width: 12 },
                    KeyDef { display: "a".to_string(), id: "a".to_string(), width: 6 },
                    KeyDef { display: "s".to_string(), id: "s".to_string(), width: 6 },
                    KeyDef { display: "d".to_string(), id: "d".to_string(), width: 6 },
                    KeyDef { display: "f".to_string(), id: "f".to_string(), width: 6 },
                    KeyDef { display: "g".to_string(), id: "g".to_string(), width: 6 },
                    KeyDef { display: "h".to_string(), id: "h".to_string(), width: 6 },
                    KeyDef { display: "j".to_string(), id: "j".to_string(), width: 6 },
                    KeyDef { display: "k".to_string(), id: "k".to_string(), width: 6 },
                    KeyDef { display: "l".to_string(), id: "l".to_string(), width: 6 },
                    KeyDef { display: ";".to_string(), id: "semicolon".to_string(), width: 6 },
                    KeyDef { display: "'".to_string(), id: "quote".to_string(), width: 6 },
                    KeyDef { display: "enter".to_string(), id: "return".to_string(), width: 18 },
                ],
                vec![
                    KeyDef { display: "shift".to_string(), id: "lshift".to_string(), width: 15 },
                    KeyDef { display: "z".to_string(), id: "z".to_string(), width: 6 },
                    KeyDef { display: "x".to_string(), id: "x".to_string(), width: 6 },
                    KeyDef { display: "c".to_string(), id: "c".to_string(), width: 6 },
                    KeyDef { display: "v".to_string(), id: "v".to_string(), width: 6 },
                    KeyDef { display: "b".to_string(), id: "b".to_string(), width: 6 },
                    KeyDef { display: "n".to_string(), id: "n".to_string(), width: 6 },
                    KeyDef { display: "m".to_string(), id: "m".to_string(), width: 6 },
                    KeyDef { display: ",".to_string(), id: "comma".to_string(), width: 6 },
                    KeyDef { display: ".".to_string(), id: "period".to_string(), width: 6 },
                    KeyDef { display: "/".to_string(), id: "slash".to_string(), width: 6 },
                    KeyDef { display: "shift".to_string(), id: "rshift".to_string(), width: 15 },
                    KeyDef { display: "up".to_string(), id: "up".to_string(), width: 6 },
                ],
                vec![
                    KeyDef { display: "fn".to_string(), id: "fn".to_string(), width: 6 },
                    KeyDef { display: "ctrl".to_string(), id: "lctrl".to_string(), width: 6 },
                    KeyDef { display: "opt".to_string(), id: "lopt".to_string(), width: 6 },
                    KeyDef { display: "cmd".to_string(), id: "lcmd".to_string(), width: 9 },
                    KeyDef { display: "space".to_string(), id: "space".to_string(), width: 33 },
                    KeyDef { display: "cmd".to_string(), id: "rcmd".to_string(), width: 9 },
                    KeyDef { display: "opt".to_string(), id: "ropt".to_string(), width: 6 },
                    KeyDef { display: "left".to_string(), id: "left".to_string(), width: 7 },
                    KeyDef { display: "down".to_string(), id: "down".to_string(), width: 7 },
                    KeyDef { display: "right".to_string(), id: "right".to_string(), width: 7 },
                ],
            ],
        }
    }

    pub fn load_keyboards() -> Vec<KeyboardConfig> {
        let dir = Self::keyboards_dir();
        let mut keyboards = Vec::new();

        if dir.exists() {
            if let Ok(entries) = fs::read_dir(&dir) {
                let mut files: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                files.sort_by_key(|e| e.file_name());
                for entry in files {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "json") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(kb) = serde_json::from_str::<KeyboardConfig>(&content) {
                                keyboards.push(kb);
                            }
                        }
                    }
                }
            }
        }

        // Если ничего не нашли, создаем дефолтные
        if keyboards.is_empty() {
            let keychron = Self::default_keychron_layout();
            let mbp = Self::default_mbp_layout();
            keyboards.push(keychron);
            keyboards.push(mbp);

            // Сохраняем дефолтные на диск
            let _ = fs::create_dir_all(&dir);
            for kb in &keyboards {
                let file_path = dir.join(format!("{}.json", kb.name));
                if let Ok(json) = serde_json::to_string_pretty(kb) {
                    let _ = fs::write(&file_path, json);
                }
            }
        }

        keyboards
    }

    pub fn save_keyboard(&self, idx: usize) -> Result<(), io::Error> {
        if idx >= self.keyboards.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid keyboard index"));
        }
        let dir = Self::keyboards_dir();
        fs::create_dir_all(&dir)?;
        let kb = &self.keyboards[idx];
        let file_path = dir.join(format!("{}.json", kb.name));
        let json = serde_json::to_string_pretty(kb)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&file_path, json)
    }

    pub fn delete_keyboard_file(&self, name: &str) -> Result<(), io::Error> {
        let dir = Self::keyboards_dir();
        let file_path = dir.join(format!("{}.json", name));
        if file_path.exists() {
            fs::remove_file(&file_path)
        } else {
            Ok(())
        }
    }

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

        let sys_shortcuts = Self::load_system_shortcuts(&home);
        items.extend(sys_shortcuts);

        for item in &mut items {
            item.search_text = format!("{} {} {} {}", item.action, item.desc, item.keys.join(" "), item.rules).to_lowercase();
        }

        items.sort_by(|a, b| {
            a.source.cmp(&b.source)
                .then_with(|| a.rules.cmp(&b.rules))
                .then_with(|| a.keys.join("+").cmp(&b.keys.join("+")))
                .then_with(|| a.desc.cmp(&b.desc))
        });

        let aliases = Self::load_app_aliases(&home);
        let keyboards = Self::load_keyboards();

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
            is_filtering_modifier: false,
            active_modifiers: std::collections::HashSet::new(),
            show_help: false,
            show_overview: false,
            config_path: Some(path),
            bulk_highlight: false,
            keyboards,
            filtered_indices: Vec::new(),
            aliases,
            status_message: None,
            selected_keyboard_idx: 0,
            keyboard_dropdown_idx: 0,
            show_keyboard_dropdown: false,
            edit_mode: EditMode::None,
            edit_selected_row: 0,
            edit_selected_col: 0,
            edit_input_buffer: String::new(),
            edit_input_field: EditField::KeyDisplay,
        };
        app.update_filtered_cache();
        app.state.select(Some(0));
        Ok(app)
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), std::time::Instant::now()));
    }

    pub fn reload(&mut self) -> Result<(), io::Error> {
        if let Some(ref path) = self.config_path {
            let content = fs::read_to_string(&path)?;
            let mut items: Vec<Shortcut> = serde_json::from_str(&content)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let sys_shortcuts = Self::load_system_shortcuts(&home);
            items.extend(sys_shortcuts);

            for item in &mut items {
                item.search_text = format!("{} {} {} {}", item.action, item.desc, item.keys.join(" "), item.rules).to_lowercase();
            }

            items.sort_by(|a, b| {
                a.source.cmp(&b.source)
                    .then_with(|| a.rules.cmp(&b.rules))
                    .then_with(|| a.keys.join("+").cmp(&b.keys.join("+")))
                    .then_with(|| a.desc.cmp(&b.desc))
            });

            self.aliases = Self::load_app_aliases(&home);
            self.items = items;
            self.update_filtered_cache();
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

    pub fn update_filtered_cache(&mut self) {
        let query = self.search_query.to_lowercase();
        let app_q = self.app_filter_query.to_lowercase();
        let filter_str = self.filter.as_str();

        self.filtered_indices = self.items
            .iter()
            .enumerate()
            .filter_map(|(idx, i)| {
                // Core Source Filter
                if self.filter != Filter::All && !i.source.contains(filter_str) {
                    return None;
                }

                // Sub-filter for App (only if in Karabiner filter)
                if self.filter == Filter::Karabiner && !app_q.is_empty() {
                    let mut matched = false;
                    for rule_tag in i.rules.split_whitespace() {
                        if rule_tag.is_empty() {
                            continue;
                        }
                        if let Some(idx) = rule_tag.rfind('_') {
                            let slug = &rule_tag[..idx];
                            // 1. Slug starts with or contains query (case-insensitive)
                            if slug.to_lowercase().contains(&app_q) {
                                matched = true;
                                break;
                            }
                            // 2. Any alias key starts with or contains query (case-insensitive) and maps to this slug
                            for (app_name, alias_slug) in &self.aliases {
                                if app_name.to_lowercase().contains(&app_q) && alias_slug.to_lowercase() == slug.to_lowercase() {
                                    matched = true;
                                    break;
                                }
                            }
                            if matched {
                                break;
                            }
                        }
                    }
                    if !matched {
                        return None;
                    }
                }
                
                // Key Filter (Space mode)
                if self.is_filtering_key {
                    if let Some(target_char) = self.key_filter {
                        let target_str = target_char.to_string().to_lowercase();
                        if !i.keys.iter().any(|k| k.to_lowercase() == target_str) {
                            return None;
                        }
                    }
                }

                // Modifier Filter (M mode)
                if self.is_filtering_modifier {
                    let mut shortcut_mods = std::collections::HashSet::new();
                    for k in &i.keys {
                        let k_lower = k.to_lowercase();
                        if k_lower.contains("cmd") || k_lower.contains("command") || k_lower == "hyper" {
                            shortcut_mods.insert("cmd".to_string());
                        }
                        if k_lower.contains("opt") || k_lower.contains("alt") || k_lower.contains("option") || k_lower == "hyper" {
                            shortcut_mods.insert("opt".to_string());
                        }
                        if k_lower.contains("ctrl") || k_lower.contains("control") || k_lower == "hyper" {
                            shortcut_mods.insert("ctrl".to_string());
                        }
                        if k_lower.contains("shift") || k_lower == "hyper" {
                            shortcut_mods.insert("shift".to_string());
                        }
                    }
                    
                    if shortcut_mods != self.active_modifiers {
                        return None;
                    }
                }

                // Text search filter
                if !query.is_empty() && !i.search_text.contains(&query) {
                    return None;
                }

                Some(idx)
            })
            .collect();
    }

    pub fn filtered_items(&self) -> Vec<&Shortcut> {
        self.filtered_indices.iter().map(|&idx| &self.items[idx]).collect()
    }

    pub fn get_normalized_signature(keys: &[String]) -> String {
        let mut normalized: Vec<String> = keys.iter()
            .map(|k| {
                let kl = k.to_lowercase();
                match kl.as_str() {
                    "command" | "lcmd" | "rcmd" => "cmd".to_string(),
                    "option" | "alt" | "lopt" | "ropt" => "opt".to_string(),
                    "control" | "lctrl" | "rctrl" => "ctrl".to_string(),
                    "lshift" | "rshift" => "shift".to_string(),
                    _ => kl,
                }
            })
            .filter(|k| !k.is_empty() && k != "-")
            .collect();
        normalized.sort();
        normalized.join("+")
    }

    pub fn analyze_source(&self, source_name: &str) -> SourceAnalysis {
        // 1. Total Shortcuts
        let source_items: Vec<&Shortcut> = self.items.iter()
            .filter(|item| item.source.contains(source_name))
            .collect();
        let total_shortcuts = source_items.len();

        // 2. Top Modifier
        let mut mod_counts = std::collections::HashMap::new();
        for item in &source_items {
            let mut has_cmd = false;
            let mut has_opt = false;
            let mut has_ctrl = false;
            let mut has_shift = false;
            
            for key in &item.keys {
                let kl = key.to_lowercase();
                if kl.contains("cmd") || kl.contains("command") || kl == "hyper" {
                    has_cmd = true;
                }
                if kl.contains("opt") || kl.contains("option") || kl.contains("alt") || kl == "hyper" {
                    has_opt = true;
                }
                if kl.contains("ctrl") || kl.contains("control") || kl == "hyper" {
                    has_ctrl = true;
                }
                if kl.contains("shift") || kl == "hyper" {
                    has_shift = true;
                }
            }
            
            if has_cmd && has_opt && has_ctrl && has_shift {
                *mod_counts.entry("HYPER".to_string()).or_insert(0) += 1;
            } else if !has_cmd && has_opt && has_ctrl && has_shift {
                *mod_counts.entry("MEH".to_string()).or_insert(0) += 1;
            } else {
                if has_cmd {
                    *mod_counts.entry("CMD".to_string()).or_insert(0) += 1;
                }
                if has_opt {
                    *mod_counts.entry("OPT".to_string()).or_insert(0) += 1;
                }
                if has_ctrl {
                    *mod_counts.entry("CTRL".to_string()).or_insert(0) += 1;
                }
                if has_shift {
                    *mod_counts.entry("SHIFT".to_string()).or_insert(0) += 1;
                }
            }
        }
        let top_modifier = mod_counts.into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(m, _)| m)
            .unwrap_or_else(|| "None".to_string());

        // 3. Config Path & Last Modified Time
        let mut final_config_path = match source_name.to_lowercase().as_str() {
            s if s.contains("skhd") => "~/.config/skhd/skhdrc".to_string(),
            s if s.contains("karabiner") => "~/.config/karabiner/karabiner.json".to_string(),
            _ => "System Settings".to_string(),
        };

        let last_modified = if final_config_path != "System Settings" {
            let home = std::env::var("HOME").unwrap_or_default();
            let mut expanded_path = final_config_path.replace("~", &home);
            
            // If skhdrc at ~/.config doesn't exist, check ~/.skhdrc
            if source_name.contains("skhd") && !std::path::Path::new(&expanded_path).exists() {
                let alt_path = format!("{}/.skhdrc", home);
                if std::path::Path::new(&alt_path).exists() {
                    final_config_path = "~/.skhdrc".to_string();
                    expanded_path = alt_path;
                }
            }

            if let Ok(metadata) = std::fs::metadata(&expanded_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        let secs = elapsed.as_secs();
                        if secs < 60 {
                            "Just now".to_string()
                        } else if secs < 3600 {
                            format!("{}m ago", secs / 60)
                        } else if secs < 86400 {
                            format!("{}h ago", secs / 3600)
                        } else {
                            format!("{}d ago", secs / 86400)
                        }
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Not Found".to_string()
            }
        } else {
            "N/A".to_string()
        };

        // 4. Conflicts
        // We compare every item in the current source with items in other sources
        let mut conflicts = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        for item in &source_items {
            let sig = Self::get_normalized_signature(&item.keys);
            if sig.is_empty() || sig == "key" {
                continue;
            }
            if seen_keys.contains(&sig) {
                continue; // Avoid duplicate listings for the same key signature
            }

            // Find other sources that contain the exact same signature
            for other_item in &self.items {
                if !other_item.source.contains(source_name) {
                    let other_sig = Self::get_normalized_signature(&other_item.keys);
                    if sig == other_sig {
                        // Extract other source name nicely (e.g. system, skhd, karabiner)
                        let other_source = other_item.source.split_whitespace()
                            .find(|&s| s == "karabiner" || s == "skhd" || s == "system")
                            .unwrap_or("other")
                            .to_string();

                        conflicts.push((sig.clone(), other_source, other_item.desc.clone()));
                        seen_keys.insert(sig.clone());
                        break; // Only record the first conflict for this signature
                    }
                }
            }
        }

        SourceAnalysis {
            total_shortcuts,
            top_modifier,
            config_path: final_config_path,
            last_modified,
            conflicts,
        }
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
            is_filtering_modifier: false,
            active_modifiers: std::collections::HashSet::new(),
            show_help: false,
            show_overview: false,
            config_path: None,
            bulk_highlight: false,
            keyboards: vec![],
            filtered_indices: vec![],
            aliases: std::collections::HashMap::new(),
            status_message: None,
            selected_keyboard_idx: 0,
            keyboard_dropdown_idx: 0,
            show_keyboard_dropdown: false,
            edit_mode: EditMode::None,
            edit_selected_row: 0,
            edit_selected_col: 0,
            edit_input_buffer: String::new(),
            edit_input_field: EditField::KeyDisplay,
        };
        app.update_filtered_cache();
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
        let mut app = App {
            state: TableState::default(),
            items: vec![
                mock_shortcut("skhd"),
                mock_shortcut("karabiner"),
                mock_shortcut("system"),
            ],
            filter: Filter::Skhd,
            search_query: String::new(),
            app_filter_query: String::new(),
            key_filter: None,
            is_searching: false,
            is_filtering_app: false,
            is_filtering_key: false,
            is_filtering_modifier: false,
            active_modifiers: std::collections::HashSet::new(),
            show_help: false,
            show_overview: false,
            config_path: None,
            bulk_highlight: false,
            keyboards: vec![],
            filtered_indices: vec![],
            aliases: std::collections::HashMap::new(),
            status_message: None,
            selected_keyboard_idx: 0,
            keyboard_dropdown_idx: 0,
            show_keyboard_dropdown: false,
            edit_mode: EditMode::None,
            edit_selected_row: 0,
            edit_selected_col: 0,
            edit_input_buffer: String::new(),
            edit_input_field: EditField::KeyDisplay,
        };
        app.update_filtered_cache();

        let filtered = app.filtered_items();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source, "skhd");
    }

    #[test]
    fn test_modifier_filtering() {
        let mut shortcut_hyper = mock_shortcut("skhd");
        shortcut_hyper.keys = vec!["cmd".into(), "opt".into(), "ctrl".into(), "shift".into(), "a".into()];
        
        let mut shortcut_meh = mock_shortcut("karabiner");
        shortcut_meh.keys = vec!["opt".into(), "ctrl".into(), "shift".into(), "b".into()];

        let mut app = App {
            state: TableState::default(),
            items: vec![shortcut_hyper, shortcut_meh],
            filter: Filter::All,
            search_query: String::new(),
            app_filter_query: String::new(),
            key_filter: None,
            is_searching: false,
            is_filtering_app: false,
            is_filtering_key: false,
            is_filtering_modifier: true,
            active_modifiers: ["cmd".to_string(), "opt".to_string(), "ctrl".to_string(), "shift".to_string()].into_iter().collect(),
            show_help: false,
            show_overview: false,
            config_path: None,
            bulk_highlight: false,
            keyboards: vec![],
            filtered_indices: vec![],
            aliases: std::collections::HashMap::new(),
            status_message: None,
            selected_keyboard_idx: 0,
            keyboard_dropdown_idx: 0,
            show_keyboard_dropdown: false,
            edit_mode: EditMode::None,
            edit_selected_row: 0,
            edit_selected_col: 0,
            edit_input_buffer: String::new(),
            edit_input_field: EditField::KeyDisplay,
        };
        app.update_filtered_cache();
        
        let filtered = app.filtered_items();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source, "skhd");

        // Switch to Meh modifiers
        app.active_modifiers = ["opt".to_string(), "ctrl".to_string(), "shift".to_string()].into_iter().collect();
        app.update_filtered_cache();

        let filtered_meh = app.filtered_items();
        assert_eq!(filtered_meh.len(), 1);
        assert_eq!(filtered_meh[0].source, "karabiner");
    }

    #[test]
    fn test_conflict_detector_and_statistics() {
        let mut shortcut_a = mock_shortcut("skhd");
        shortcut_a.keys = vec!["cmd".into(), "shift".into(), "a".into()];
        
        let mut shortcut_b = mock_shortcut("karabiner");
        shortcut_b.keys = vec!["shift".into(), "cmd".into(), "a".into()]; // Same normalized keys signature!

        let app = App {
            state: TableState::default(),
            items: vec![shortcut_a, shortcut_b],
            filter: Filter::All,
            search_query: String::new(),
            app_filter_query: String::new(),
            key_filter: None,
            is_searching: false,
            is_filtering_app: false,
            is_filtering_key: false,
            is_filtering_modifier: false,
            active_modifiers: std::collections::HashSet::new(),
            show_help: false,
            show_overview: false,
            config_path: None,
            bulk_highlight: false,
            keyboards: vec![],
            filtered_indices: vec![],
            aliases: std::collections::HashMap::new(),
            status_message: None,
            selected_keyboard_idx: 0,
            keyboard_dropdown_idx: 0,
            show_keyboard_dropdown: false,
            edit_mode: EditMode::None,
            edit_selected_row: 0,
            edit_selected_col: 0,
            edit_input_buffer: String::new(),
            edit_input_field: EditField::KeyDisplay,
        };

        // Normalize keys test
        let sig = App::get_normalized_signature(&["shift".to_string(), "cmd".to_string(), "a".to_string()]);
        assert_eq!(sig, "a+cmd+shift");

        // Conflict check
        let analysis_skhd = app.analyze_source("skhd");
        assert_eq!(analysis_skhd.total_shortcuts, 1);
        assert_eq!(analysis_skhd.conflicts.len(), 1);
        assert_eq!(analysis_skhd.conflicts[0].0, "a+cmd+shift");
        assert_eq!(analysis_skhd.conflicts[0].1, "karabiner");
    }

    #[test]
    fn test_top_modifier_hyper_meh() {
        let mut shortcut_a = mock_shortcut("skhd");
        shortcut_a.keys = vec!["cmd".into(), "opt".into(), "ctrl".into(), "shift".into(), "a".into()]; // Hyper

        let mut shortcut_b = mock_shortcut("skhd");
        shortcut_b.keys = vec!["opt".into(), "ctrl".into(), "shift".into(), "b".into()]; // Meh

        let mut app = App {
            state: TableState::default(),
            items: vec![shortcut_a, shortcut_b.clone(), shortcut_b], // 1 Hyper, 2 Meh
            filter: Filter::All,
            search_query: String::new(),
            app_filter_query: String::new(),
            key_filter: None,
            is_searching: false,
            is_filtering_app: false,
            is_filtering_key: false,
            is_filtering_modifier: false,
            active_modifiers: std::collections::HashSet::new(),
            show_help: false,
            show_overview: false,
            config_path: None,
            bulk_highlight: false,
            keyboards: vec![],
            filtered_indices: vec![],
            aliases: std::collections::HashMap::new(),
            status_message: None,
            selected_keyboard_idx: 0,
            keyboard_dropdown_idx: 0,
            show_keyboard_dropdown: false,
            edit_mode: EditMode::None,
            edit_selected_row: 0,
            edit_selected_col: 0,
            edit_input_buffer: String::new(),
            edit_input_field: EditField::KeyDisplay,
        };

        let analysis = app.analyze_source("skhd");
        assert_eq!(analysis.top_modifier, "MEH");

        // Add more Hypers to make HYPER the top modifier
        let mut shortcut_c = mock_shortcut("skhd");
        shortcut_c.keys = vec!["cmd".into(), "opt".into(), "ctrl".into(), "shift".into(), "c".into()]; // Hyper
        app.items.push(shortcut_c.clone());
        app.items.push(shortcut_c);

        let analysis_updated = app.analyze_source("skhd");
        assert_eq!(analysis_updated.top_modifier, "HYPER");
    }

    #[test]
    fn test_app_filtering_aliases() {
        let mut shortcut_ghostty = mock_shortcut("karabiner");
        shortcut_ghostty.rules = "gh_d".to_string();

        let mut shortcut_xcode = mock_shortcut("karabiner");
        shortcut_xcode.rules = "xc_d".to_string();

        let mut aliases = std::collections::HashMap::new();
        aliases.insert("ghostty".to_string(), "gh".to_string());
        aliases.insert("xcode".to_string(), "xc".to_string());

        let mut app = App {
            state: TableState::default(),
            items: vec![shortcut_ghostty, shortcut_xcode],
            filter: Filter::Karabiner,
            search_query: String::new(),
            app_filter_query: String::new(),
            key_filter: None,
            is_searching: false,
            is_filtering_app: true,
            is_filtering_key: false,
            is_filtering_modifier: false,
            active_modifiers: std::collections::HashSet::new(),
            show_help: false,
            show_overview: false,
            config_path: None,
            bulk_highlight: false,
            keyboards: vec![],
            filtered_indices: vec![],
            aliases,
            status_message: None,
            selected_keyboard_idx: 0,
            keyboard_dropdown_idx: 0,
            show_keyboard_dropdown: false,
            edit_mode: EditMode::None,
            edit_selected_row: 0,
            edit_selected_col: 0,
            edit_input_buffer: String::new(),
            edit_input_field: EditField::KeyDisplay,
        };

        // Query "g" should match ghostty (gh_d) but not xcode (xc_d)
        app.app_filter_query = "g".to_string();
        app.update_filtered_cache();
        let filtered_g = app.filtered_items();
        assert_eq!(filtered_g.len(), 1);
        assert_eq!(filtered_g[0].rules, "gh_d");

        // Query "ghostty" should match ghostty (gh_d) via alias
        app.app_filter_query = "ghostty".to_string();
        app.update_filtered_cache();
        let filtered_ghostty = app.filtered_items();
        assert_eq!(filtered_ghostty.len(), 1);
        assert_eq!(filtered_ghostty[0].rules, "gh_d");

        // Query "x" should match xcode (xc_d) but not ghostty
        app.app_filter_query = "x".to_string();
        app.update_filtered_cache();
        let filtered_x = app.filtered_items();
        assert_eq!(filtered_x.len(), 1);
        assert_eq!(filtered_x[0].rules, "xc_d");
    }
}
