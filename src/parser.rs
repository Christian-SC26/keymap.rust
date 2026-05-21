use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use regex::Regex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RawShortcut {
    pub source: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rules: String,
    pub keys: Vec<String>,
    pub action: String,
    pub desc: String,
}

struct ShortcutEntry {
    source: String,
    rules: String,
    action: String,
    description: String,
}

struct ShortcutData {
    entries: Vec<ShortcutEntry>,
}

fn load_app_aliases() -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    
    let alias_paths = [
        "app_aliases.json".to_string(),
        "src/app_aliases.json".to_string(),
        format!("{}/.config/karabiner/app_aliases.json", home),
    ];

    for path in &alias_paths {
        if let Ok(c) = fs::read_to_string(path) {
            if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(&c) {
                aliases = parsed;
                break;
            }
        }
    }
    aliases
}

pub fn run_parser(output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut all_shortcuts: HashMap<String, ShortcutData> = HashMap::new();

    let home = std::env::var("HOME")?;
    let aliases = load_app_aliases();
    
    let karabiner_path = format!("{}/.config/karabiner/karabiner.json", home);
    if let Err(e) = parse_karabiner_json(&karabiner_path, &mut all_shortcuts, &aliases) {
        eprintln!("[Warn] Error parsing Karabiner: {}", e);
    }

    let skhd_path = format!("{}/.skhdrc", home);
    if let Err(e) = parse_skhd_config(&skhd_path, &mut all_shortcuts) {
        eprintln!("[Warn] Error parsing skhd: {}", e);
    }

    export_json(all_shortcuts, output_path)?;
    Ok(())
}

fn format_key(key_str: &str) -> String {
    if key_str.is_empty() {
        return String::new();
    }

    let k = key_str.to_lowercase();
    match k.as_str() {
        "left_command" | "lcmd" | "lcommand" => "lcmd".to_string(),
        "right_command" | "rcmd" | "rcommand" => "rcmd".to_string(),
        "command" | "cmd" => "cmd".to_string(),
        
        "left_option" | "lalt" | "lopt" | "loption" => "lopt".to_string(),
        "right_option" | "ralt" | "ropt" | "roption" => "ropt".to_string(),
        "option" | "alt" | "opt" => "opt".to_string(),
        
        "left_control" | "lctrl" | "lcontrol" => "lctrl".to_string(),
        "right_control" | "rctrl" | "rcontrol" => "rctrl".to_string(),
        "control" | "ctrl" => "ctrl".to_string(),
        
        "left_shift" | "lshift" => "lshift".to_string(),
        "right_shift" | "rshift" => "rshift".to_string(),
        "shift" => "shift".to_string(),
        
        "hyper" => "hyper".to_string(),

        "vk_none" => "-".to_string(),
        "return" | "enter" => "return".to_string(),
        "space" | "spacebar" => "space".to_string(),
        "escape" => "esc".to_string(),
        "tab" => "tab".to_string(),
        "caps_lock" => "caps".to_string(),
        "delete_or_backspace" => "backspace".to_string(),
        "delete_forward" => "del".to_string(),

        "left_arrow" => "left".to_string(),
        "right_arrow" => "right".to_string(),
        "up_arrow" => "up".to_string(),
        "down_arrow" => "down".to_string(),
        "page_up" => "pgup".to_string(),
        "page_down" => "pgdn".to_string(),
        "home" => "home".to_string(),
        "end" => "end".to_string(),

        "grave_accent_and_tilde" | "0x32" => "ˋ".to_string(),
        "hyphen" | "0x1b" => "-".to_string(),
        "equal_sign" | "0x18" => "=".to_string(),
        "open_bracket" | "0x21" => "[".to_string(),
        "close_bracket" | "0x1e" => "]".to_string(),
        "backslash" | "0x2a" => "\\".to_string(),
        "semicolon" | "0x29" => ";".to_string(),
        "quote" | "0x27" => "'".to_string(),
        "comma" | "0x2b" => ",".to_string(),
        "period" | "0x2f" => ".".to_string(),
        "slash" | "0x2c" => "/".to_string(),

        "play_or_pause" => "play/pause".to_string(),
        "mute" => "mute".to_string(),
        "volume_decrement" => "vol_down".to_string(),
        "volume_increment" => "vol_up".to_string(),
        "display_brightness_decrement" => "br_down".to_string(),
        "display_brightness_increment" => "br_up".to_string(),
        
        other => {
            if other.starts_with('f') && other[1..].parse::<u8>().is_ok() {
                other.to_uppercase()
            } else {
                other.to_string()
            }
        }
    }
}

fn process_modifiers(mods: &[String]) -> String {
    if mods.is_empty() {
        return String::new();
    }

    let mut bases = HashSet::new();
    for m in mods {
        if m.contains("cmd") { bases.insert("cmd"); }
        else if m.contains("opt") { bases.insert("opt"); }
        else if m.contains("ctrl") { bases.insert("ctrl"); }
        else if m.contains("shift") { bases.insert("shift"); }
        else if m == "hyper" {
            bases.insert("cmd"); bases.insert("opt"); bases.insert("ctrl"); bases.insert("shift");
        }
    }

    if bases.len() == 4 && bases.contains("cmd") && bases.contains("opt") && bases.contains("ctrl") && bases.contains("shift") {
        return "hyper".to_string();
    }

    let mut sorted_mods = mods.to_vec();
    sorted_mods.sort_by_key(|m| {
        if m.contains("cmd") { 1 }
        else if m.contains("opt") { 2 }
        else if m.contains("ctrl") { 3 }
        else if m.contains("shift") { 4 }
        else { 5 }
    });

    sorted_mods.join(" + ")
}

fn parse_action_array(to_array: &[serde_json::Value]) -> Vec<String> {
    let mut actions = Vec::new();
    for t in to_array {
        if let Some(t_key_code) = t.get("key_code").or_else(|| t.get("consumer_key_code")).and_then(|v| v.as_str()) {
            let t_key = format_key(t_key_code);
            if let Some(to_mods_array) = t.get("modifiers").and_then(|m| m.as_array()) {
                let t_mods_list: Vec<String> = to_mods_array.iter().filter_map(|m| m.as_str()).map(format_key).collect();
                let t_mods = process_modifiers(&t_mods_list);
                if t_mods.is_empty() {
                    actions.push(t_key);
                } else {
                    actions.push(format!("{} + {}", t_mods, t_key));
                }
            } else {
                actions.push(t_key);
            }
        }
    }
    actions
}

fn parse_karabiner_json(path: &str, shortcuts: &mut HashMap<String, ShortcutData>, aliases: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(profiles) = v.get("profiles").and_then(|p| p.as_array()) {
        for profile in profiles {
            if let Some(rules) = profile.get("complex_modifications").and_then(|cm| cm.get("rules")).and_then(|r| r.as_array()) {
                for rule in rules {
                    let description = rule.get("description").and_then(|d| d.as_str()).unwrap_or("-").to_string();

                    if let Some(manipulators) = rule.get("manipulators").and_then(|m| m.as_array()) {
                        for manip in manipulators {
                            if manip.get("type").and_then(|t| t.as_str()) != Some("basic") {
                                continue;
                            }

                            let from = manip.get("from").cloned().unwrap_or(serde_json::Value::Object(Default::default()));
                            let key_code = from.get("key_code")
                                .or_else(|| from.get("consumer_key_code"))
                                .or_else(|| from.get("pointing_button"))
                                .or_else(|| from.get("any"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            
                            let key = format_key(key_code);
                            
                            let mandatory_mods = from.get("modifiers").and_then(|m| m.get("mandatory"));
                            let mods = if let Some(m_array) = mandatory_mods.and_then(|m| m.as_array()) {
                                let m_list: Vec<String> = m_array.iter().filter_map(|m| m.as_str()).map(format_key).collect();
                                process_modifiers(&m_list)
                            } else {
                                String::new()
                            };

                            let optional_mods = from.get("modifiers").and_then(|m| m.get("optional"));
                            let mut opt_str = String::new();
                            if let Some(opt_array) = optional_mods.and_then(|m| m.as_array()) {
                                let opt_list: Vec<String> = opt_array.iter().filter_map(|m| m.as_str()).map(|s| s.to_string()).collect();
                                if opt_list.iter().any(|s| s == "any") {
                                    opt_str = " (+ any)".to_string();
                                } else if !opt_list.is_empty() {
                                    let formatted_opt_list: Vec<String> = opt_list.into_iter().map(|s| format_key(&s)).collect();
                                    opt_str = format!(" (+ {})", formatted_opt_list.join(" / "));
                                }
                            }

                            let trigger_base = if !mods.is_empty() && !key.is_empty() {
                                format!("{} + {}", mods, key)
                            } else if !key.is_empty() {
                                key
                            } else if !mods.is_empty() {
                                mods
                            } else {
                                "-".to_string()
                            };

                            let trigger = if !opt_str.is_empty() && trigger_base != "-" {
                                format!("{}{}", trigger_base, opt_str)
                            } else {
                                trigger_base
                            };

                            if trigger == "-" 
                                && manip.get("to").is_none()
                                && manip.get("to_if_alone").is_none()
                                && manip.get("to_if_held_down").is_none()
                                && manip.get("to_after_key_up").is_none()
                            {
                                continue;
                            }

                            // Определение приложения (enable/disable)
                            let mut tags = Vec::new();
                            if let Some(conditions) = manip.get("conditions").and_then(|c| c.as_array()) {
                                for cond in conditions {
                                    let ctype = cond.get("type").and_then(|t| t.as_str());
                                    let is_enable = ctype == Some("frontmost_application_if");
                                    let is_disable = ctype == Some("frontmost_application_unless");
                                    
                                    if let Some(bundles) = cond.get("bundle_identifiers").and_then(|b| b.as_array()).filter(|_| is_enable || is_disable) {
                                        for b in bundles {
                                            if let Some(b_str) = b.as_str() {
                                                let app_slug = get_app_slug(b_str, aliases);
                                                let suffix = if is_enable { "_e" } else { "_d" };
                                                tags.push(format!("{}{}", app_slug, suffix));
                                            }
                                        }
                                    }
                                }
                            }
                            
                            let rules_str = tags.join(" ");

                            let mut action_parts = Vec::new();

                            if let Some(to_array) = manip.get("to").and_then(|t| t.as_array()) {
                                let to_actions = parse_action_array(to_array);
                                if !to_actions.is_empty() {
                                    action_parts.push(to_actions.join(" "));
                                }
                            }

                            if let Some(alone_array) = manip.get("to_if_alone").and_then(|t| t.as_array()) {
                                let alone_actions = parse_action_array(alone_array);
                                if !alone_actions.is_empty() {
                                    action_parts.push(format!("{} (tapped)", alone_actions.join(" ")));
                                }
                            }

                            if let Some(held_array) = manip.get("to_if_held_down").and_then(|t| t.as_array()) {
                                let held_actions = parse_action_array(held_array);
                                if !held_actions.is_empty() {
                                    action_parts.push(format!("{} (held)", held_actions.join(" ")));
                                }
                            }

                            if let Some(released_array) = manip.get("to_after_key_up").and_then(|t| t.as_array()) {
                                let released_actions = parse_action_array(released_array);
                                if !released_actions.is_empty() {
                                    action_parts.push(format!("{} (released)", released_actions.join(" ")));
                                }
                            }

                            let action_str = if action_parts.is_empty() { "-".to_string() } else { action_parts.join(" / ") };
                            add_to_dict(shortcuts, "karabiner", &rules_str, &trigger, &action_str, &description);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_app_slug(bundle_id: &str, aliases: &HashMap<String, String>) -> String {
    let parts: Vec<&str> = bundle_id.split('.').collect();
    let name = parts.last().unwrap_or(&bundle_id).to_lowercase();
    if let Some(slug) = aliases.get(&name) {
        slug.clone()
    } else {
        name.chars().filter(|c| c.is_alphanumeric()).take(2).collect()
    }
}

fn parse_skhd_trigger(trigger_raw: &str) -> String {
    let parts: Vec<&str> = trigger_raw.split(['+', '-']).collect();
    let formatted_parts: Vec<String> = parts.iter().map(|p| p.trim()).filter(|p| !p.is_empty()).map(format_key).collect();

    if formatted_parts.len() > 1 {
        let mods_list = &formatted_parts[..formatted_parts.len() - 1];
        let key = &formatted_parts[formatted_parts.len() - 1];
        let mods = process_modifiers(mods_list);
        if mods.is_empty() { key.clone() } else { format!("{} + {}", mods, key) }
    } else if !formatted_parts.is_empty() {
        formatted_parts[0].clone()
    } else {
        "-".to_string()
    }
}

fn parse_skhd_config(path: &str, shortcuts: &mut HashMap<String, ShortcutData>) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    let lines = content.lines();

    let mut in_block = false;
    let mut current_trigger = "-".to_string();
    let mut block_actions = Vec::new();

    let re_comment = Regex::new(r"--.+").unwrap();

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(stripped) = line.strip_suffix('[') {
            in_block = true;
            let mut trigger_raw = stripped.trim();
            if let Some(s) = trigger_raw.strip_suffix(':') {
                trigger_raw = s;
            } else if let Some(s) = trigger_raw.strip_suffix("->") {
                trigger_raw = s;
            }
            current_trigger = parse_skhd_trigger(trigger_raw.trim());
            block_actions = Vec::new();
            continue;
        }

        if in_block {
            if line == "]" {
                in_block = false;
                if !block_actions.is_empty() {
                    let description = block_actions.join(" | ");
                    add_to_dict(shortcuts, "skhd", "", &current_trigger, "-", &description);
                }
                continue;
            }

            if let Some(stripped) = line.strip_suffix('~') {
                let app_raw = stripped.trim();
                let app_name = if app_raw == "*" { "Остальные".to_string() } else { app_raw.trim_matches(|c| c == '"' || c == '\'').to_string() };
                block_actions.push(format!("{}: pass-through (~)", app_name));
                continue;
            }

            if line.contains(':') || line.contains("->") {
                let separator = if line.contains(':') { ":" } else { "->" };
                let parts: Vec<&str> = line.splitn(2, separator).collect();
                let app_raw = parts[0].trim();
                let action_raw = parts[1].trim();
                
                let action_desc = re_comment.find(action_raw).map(|m| m.as_str()).unwrap_or(action_raw);
                let app_name = if app_raw == "*" { "Остальные".to_string() } else { app_raw.trim_matches(|c| c == '"' || c == '\'').to_string() };
                block_actions.push(format!("{}: {}", app_name, action_desc));
            }
            continue;
        }

        if !line.contains(':') && !line.contains("->") {
            continue;
        }

        let separator = if line.contains(':') { ":" } else { "->" };
        let parts: Vec<&str> = line.splitn(2, separator).collect();
        let trigger_raw = parts[0].trim();
        let action_raw = parts[1].trim();

        let trigger = parse_skhd_trigger(trigger_raw);
        let description = re_comment.find(action_raw).map(|m| m.as_str()).unwrap_or(action_raw);

        add_to_dict(shortcuts, "skhd sk", "", &trigger, "-", description);
    }

    Ok(())
}

fn add_to_dict(shortcuts: &mut HashMap<String, ShortcutData>, source: &str, rules: &str, trigger: &str, action: &str, description: &str) {
    let data = shortcuts.entry(trigger.to_string()).or_insert_with(|| ShortcutData {
        entries: Vec::new(),
    });

    data.entries.push(ShortcutEntry {
        source: source.to_string(),
        rules: rules.to_string(),
        action: action.to_string(),
        description: description.to_string(),
    });
}

fn export_json(shortcuts: HashMap<String, ShortcutData>, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut output_data = Vec::new();

    for (trigger, data) in shortcuts {
        let mut sources = HashSet::new();
        let mut rules_set = HashSet::new();
        
        for entry in &data.entries {
            sources.insert(entry.source.clone());
            if !entry.rules.is_empty() {
                rules_set.insert(entry.rules.clone());
            }
        }

        let mut sources_vec: Vec<String> = sources.into_iter().collect();
        sources_vec.sort();
        let source_str = sources_vec.join(" | ");

        let mut rules_vec: Vec<String> = rules_set.into_iter().collect();
        rules_vec.sort();
        let rules_str = rules_vec.join(" | ");

        let mut clean_actions = Vec::new();
        for entry in &data.entries {
            if entry.action != "-" && !entry.action.is_empty() {
                clean_actions.push(entry.action.clone());
            }
        }
        let action_str = if clean_actions.is_empty() { "-".to_string() } else { clean_actions.join(" | ") };

        let mut clean_descs = Vec::new();
        for entry in &data.entries {
            if entry.description != "-" && !entry.description.is_empty() {
                clean_descs.push(entry.description.clone());
            }
        }
        let desc_str = if clean_descs.is_empty() { "-".to_string() } else { clean_descs.join(" | ") };

        let keys = if trigger != "-" {
            trigger.split('+')
                .map(|s| s.trim().to_string())
                .filter(|k| k != "vk_none" && !k.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        output_data.push(RawShortcut {
            source: source_str,
            rules: rules_str,
            keys,
            action: action_str,
            desc: desc_str,
        });
    }

    let f = fs::File::create(output_path)?;
    serde_json::to_writer_pretty(f, &output_data)?;
    println!("Успешно! JSON сохранен по пути: {}", output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_action_array() {
        let json_str = r#"[
            {
                "key_code": "escape"
            },
            {
                "key_code": "tab",
                "modifiers": ["left_shift", "left_command"]
            }
        ]"#;
        let val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let arr = val.as_array().unwrap();
        let parsed = parse_action_array(arr);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], "esc");
        assert_eq!(parsed[1], "lcmd + lshift + tab");
    }

    #[test]
    fn test_parse_karabiner_complex_modifications() {
        let json_content = r#"{
            "profiles": [
                {
                    "name": "Default profile",
                    "complex_modifications": {
                        "rules": [
                            {
                                "description": "Caps Lock to Hyper or Escape",
                                "manipulators": [
                                    {
                                        "type": "basic",
                                        "from": {
                                            "key_code": "caps_lock",
                                            "modifiers": {
                                                "optional": ["any"]
                                            }
                                        },
                                        "to": [
                                            {
                                                "key_code": "left_shift",
                                                "modifiers": ["left_command", "left_control", "left_option"]
                                            }
                                        ],
                                        "to_if_alone": [
                                            {
                                                "key_code": "escape"
                                            }
                                        ],
                                        "to_if_held_down": [
                                            {
                                                "key_code": "tab"
                                            }
                                        ]
                                    }
                                ]
                            }
                        ]
                    }
                }
            ]
        }"#;

        let mut shortcuts = HashMap::new();
        let aliases = HashMap::new();
        
        let path = "test_karabiner_mock.json";
        fs::write(path, json_content).unwrap();
        
        let res = parse_karabiner_json(path, &mut shortcuts, &aliases);
        fs::remove_file(path).ok();
        
        assert!(res.is_ok());
        
        let trigger_key = "caps (+ any)";
        assert!(shortcuts.contains_key(trigger_key));
        
        let data = shortcuts.get(trigger_key).unwrap();
        assert_eq!(data.entries.len(), 1);
        let entry = &data.entries[0];
        assert_eq!(entry.description, "Caps Lock to Hyper or Escape");
        
        assert!(entry.action.contains("esc (tapped)"));
        assert!(entry.action.contains("tab (held)"));
    }
}
