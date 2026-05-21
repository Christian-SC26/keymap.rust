use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;
use std::io;

mod app;
mod parser;
mod ui;

use crate::app::{App, Filter, EditMode, EditField, KeyDef};
use crate::ui::ui;

// --- Главный цикл ---
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Проверка аргумента --parse
    if args.iter().any(|arg| arg == "--parse") {
        let output_path = "src/shortcuts.json".to_string();
        if let Err(e) = parser::run_parser(&output_path) {
            eprintln!("Error running parser: {}", e);
            std::process::exit(1);
        }
        println!("Parser finished successfully.");
        return Ok(());
    }

    let custom_path = args.get(1).cloned();

    let mut terminal = ratatui::init();
    let app_result = App::new(custom_path);

    let mut app = match app_result {
        Ok(app) => app,
        Err(e) => {
            ratatui::restore();
            eprintln!("Error initializing application: {}", e);
            eprintln!("Usage: shortcuts_tui [path/to/shortcuts.json]");
            return Err(e);
        }
    };

    let res = run_app(&mut terminal, &mut app);
    ratatui::restore();
    res
}

fn run_app(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        app.update_filtered_cache();
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Edit mode handlers (highest priority)
            match app.edit_mode {
                EditMode::Visual => {
                    handle_visual_edit_input(app, key);
                    continue;
                }
                EditMode::KeyInput => {
                    handle_key_input_events(app, key);
                    continue;
                }
                EditMode::KeyboardNameInput => {
                    handle_keyboard_name_input_events(app, key);
                    continue;
                }
                EditMode::None => {}
            }

            if app.show_keyboard_dropdown {
                handle_keyboard_dropdown_input(app, key);
                continue;
            }

            if app.show_help {
                app.show_help = false;
                continue;
            }

            if app.is_searching {
                handle_search_input(app, key);
            } else if app.is_filtering_app {
                handle_app_filter_input(app, key);
            } else if app.is_filtering_key {
                handle_key_filter_input(app, key);
            } else if app.is_filtering_modifier {
                if handle_modifier_filter_input(terminal, app, key)? {
                    return Ok(());
                }
            } else {
                if handle_navigation_input(terminal, app, key)? {
                    return Ok(());
                }
            }
        }
    }
}

fn handle_keyboard_dropdown_input(app: &mut App, key: event::KeyEvent) {
    let code = match key.code {
        KeyCode::Char(c) => KeyCode::Char(translate_char(c)),
        _ => key.code,
    };

    match code {
        KeyCode::Esc => {
            app.show_keyboard_dropdown = false;
        }
        KeyCode::Enter => {
            app.selected_keyboard_idx = app.keyboard_dropdown_idx;
            app.show_keyboard_dropdown = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !app.keyboards.is_empty() {
                if app.keyboard_dropdown_idx > 0 {
                    app.keyboard_dropdown_idx -= 1;
                } else {
                    app.keyboard_dropdown_idx = app.keyboards.len() - 1;
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.keyboards.is_empty() {
                if app.keyboard_dropdown_idx < app.keyboards.len() - 1 {
                    app.keyboard_dropdown_idx += 1;
                } else {
                    app.keyboard_dropdown_idx = 0;
                }
            }
        }
        // Clone (add) keyboard
        KeyCode::Char('a') => {
            if !app.keyboards.is_empty() {
                let source_idx = app.keyboard_dropdown_idx;
                let cloned = app.keyboards[source_idx].clone();
                app.edit_input_buffer = format!("{} (Copy)", cloned.name);
                app.edit_input_field = EditField::KeyboardName;
                app.edit_mode = EditMode::KeyboardNameInput;
                app.show_keyboard_dropdown = false;
            }
        }
        // Delete keyboard
        KeyCode::Char('d') => {
            if app.keyboards.len() > 1 {
                let idx = app.keyboard_dropdown_idx;
                let name = app.keyboards[idx].name.clone();
                let _ = app.delete_keyboard_file(&name);
                app.keyboards.remove(idx);
                if app.selected_keyboard_idx >= app.keyboards.len() {
                    app.selected_keyboard_idx = app.keyboards.len() - 1;
                }
                if app.keyboard_dropdown_idx >= app.keyboards.len() {
                    app.keyboard_dropdown_idx = app.keyboards.len() - 1;
                }
                app.set_status(&format!("Keyboard '{}' deleted", name));
            } else {
                app.set_status("Cannot delete the last keyboard");
            }
        }
        // Edit keyboard layout
        KeyCode::Char('e') => {
            if !app.keyboards.is_empty() {
                app.selected_keyboard_idx = app.keyboard_dropdown_idx;
                app.edit_mode = EditMode::Visual;
                app.edit_selected_row = 0;
                app.edit_selected_col = 0;
                app.show_keyboard_dropdown = false;
            }
        }
        // Rename keyboard
        KeyCode::Char('r') => {
            if !app.keyboards.is_empty() {
                let idx = app.keyboard_dropdown_idx;
                app.edit_input_buffer = app.keyboards[idx].name.clone();
                app.edit_input_field = EditField::KeyDisplay; // reuse as "rename" marker
                app.edit_mode = EditMode::KeyboardNameInput;
                app.show_keyboard_dropdown = false;
            }
        }
        _ => {}
    }
}

/// Транслирует символ из другой раскладки в эквивалентную клавишу QWERTY.
fn translate_char(c: char) -> char {
    let lower_c = c.to_lowercase().next().unwrap_or(c);
    let translated = match lower_c {
        // Русская раскладка (Безопасно, только буквы)
        'й' => 'q', 'ц' => 'w', 'у' => 'e', 'к' => 'r', 'е' => 't', 'н' => 'y', 'г' => 'u', 'ш' => 'i', 'щ' => 'o', 'з' => 'p', 'х' => '[', 'ъ' => ']',
        'ф' => 'a', 'ы' => 's', 'в' => 'd', 'а' => 'f', 'п' => 'g', 'р' => 'h', 'о' => 'j', 'л' => 'k', 'д' => 'l', 'ж' => ';', 'э' => '\'',
        'я' => 'z', 'ч' => 'x', 'с' => 'c', 'м' => 'v', 'и' => 'b', 'т' => 'n', 'ь' => 'm', 'б' => ',', 'ю' => '.',
        _ => lower_c,
    };

    if c.is_uppercase() {
        translated.to_uppercase().next().unwrap_or(translated)
    } else {
        translated
    }
}

fn handle_modifier_filter_input(terminal: &mut DefaultTerminal, app: &mut App, key: event::KeyEvent) -> io::Result<bool> {
    let code = match key.code {
        KeyCode::Char(c) => KeyCode::Char(translate_char(c)),
        _ => key.code,
    };

    match code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('m') => {
            app.is_filtering_modifier = false;
            app.active_modifiers.clear();
            app.bulk_highlight = false;
            Ok(false)
        }
        KeyCode::Char('c') | KeyCode::Char('o') | KeyCode::Char('s') | KeyCode::Char('t') => {
            let mod_str = match code {
                KeyCode::Char('c') => Some("cmd"),
                KeyCode::Char('o') => Some("opt"),
                KeyCode::Char('s') => Some("shift"),
                KeyCode::Char('t') => Some("ctrl"),
                _ => None,
            };
            if let Some(m) = mod_str {
                let m_string = m.to_string();
                if app.active_modifiers.contains(&m_string) {
                    app.active_modifiers.remove(&m_string);
                } else {
                    app.active_modifiers.insert(m_string);
                }
                app.state.select(Some(0));
                app.bulk_highlight = true;
            }
            Ok(false)
        }
        KeyCode::Char('h') => {
            let hyper_mods = ["cmd", "opt", "ctrl", "shift"];
            let has_all = hyper_mods.iter().all(|m| app.active_modifiers.contains(*m));
            if has_all {
                for m in &hyper_mods {
                    app.active_modifiers.remove(*m);
                }
            } else {
                for m in &hyper_mods {
                    app.active_modifiers.insert(m.to_string());
                }
            }
            app.state.select(Some(0));
            app.bulk_highlight = true;
            Ok(false)
        }
        KeyCode::Char('n') => {
            let meh_mods = ["opt", "ctrl", "shift"];
            let has_all_meh = meh_mods.iter().all(|m| app.active_modifiers.contains(*m));
            let has_cmd = app.active_modifiers.contains("cmd");
            if has_all_meh && !has_cmd {
                for m in &meh_mods {
                    app.active_modifiers.remove(*m);
                }
            } else {
                for m in &meh_mods {
                    app.active_modifiers.insert(m.to_string());
                }
                app.active_modifiers.remove("cmd");
            }
            app.state.select(Some(0));
            app.bulk_highlight = true;
            Ok(false)
        }
        _ => handle_navigation_input(terminal, app, key),
    }
}

fn handle_key_filter_input(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.is_filtering_key = false;
        }
        KeyCode::Char(' ') => {
            app.is_filtering_key = false;
            app.key_filter = None;
            app.state.select(Some(0));
        }
        KeyCode::Char(c) => {
            let translated = translate_char(c);
            app.key_filter = Some(translated);
            app.state.select(Some(0));
            app.bulk_highlight = false;
        }
        _ => {}
    }
}

fn handle_app_filter_input(app: &mut App, key: event::KeyEvent) {
    let code = match key.code {
        KeyCode::Char(c) => KeyCode::Char(translate_char(c)),
        _ => key.code,
    };
    match code {
        KeyCode::Enter | KeyCode::Esc => {
            app.is_filtering_app = false;
        }
        KeyCode::Char('3') => {
            app.is_filtering_app = false;
            app.app_filter_query.clear();
            app.state.select(Some(0));
        }
        KeyCode::Char(c) => {
            app.app_filter_query.push(c);
            app.state.select(Some(0));
            app.bulk_highlight = false;
        }
        KeyCode::Backspace => {
            app.app_filter_query.pop();
            app.state.select(Some(0));
            app.bulk_highlight = false;
        }
        _ => {}
    }
}

fn handle_search_input(app: &mut App, key: event::KeyEvent) {
    let code = match key.code {
        KeyCode::Char(c) => KeyCode::Char(translate_char(c)),
        _ => key.code,
    };
    match code {
        KeyCode::Enter | KeyCode::Esc => {
            app.is_searching = false;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.state.select(Some(0));
            app.bulk_highlight = false;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.state.select(Some(0));
            app.bulk_highlight = false;
        }
        _ => {}
    }
}

fn handle_navigation_input(terminal: &mut DefaultTerminal, app: &mut App, key: event::KeyEvent) -> io::Result<bool> {
    let code = match key.code {
        KeyCode::Char(c) => KeyCode::Char(translate_char(c)),
        _ => key.code,
    };

    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('r') => {
            let _ = app.reload();
            app.set_status("Success! Reloaded JSON database");
            let _ = terminal.clear(); // Clear terminal completely to fix doubling/ghosting
        }
        KeyCode::Char('p') => {
            if let Some(ref path) = app.config_path {
                let _ = parser::run_parser(path);
                let _ = app.reload(); // Instantly reload list from the newly parsed JSON
                app.set_status("Success! Parsed and reloaded configurations");
                let _ = terminal.clear(); // Clear terminal completely to fix doubling/ghosting
            }
        }
        KeyCode::Char('s') => { app.sort_shortcuts(); }
        KeyCode::Char('?') => { app.show_help = true; }
        KeyCode::Char('/') => { app.is_searching = true; }
        KeyCode::Char('o') => { app.show_overview = !app.show_overview; }
        KeyCode::Char('K') => {
            app.show_keyboard_dropdown = true;
            app.keyboard_dropdown_idx = app.selected_keyboard_idx;
            app.bulk_highlight = false;
        }
        
        KeyCode::Char(' ') => {
            app.is_filtering_key = true;
            app.key_filter = None; // Ожидаем нажатия буквы
        }

        KeyCode::Char('m') => {
            app.is_filtering_modifier = true;
            app.active_modifiers.clear();
            app.state.select(Some(0));
        }

        KeyCode::Esc if !app.search_query.is_empty() => {
            app.search_query.clear();
            app.state.select(Some(0));
            app.bulk_highlight = false;
        }

        KeyCode::Down | KeyCode::Char('j') => { app.next(); app.bulk_highlight = false; }
        KeyCode::Up | KeyCode::Char('k') => { app.previous(); app.bulk_highlight = false; }
        KeyCode::Char('d') => { app.jump_down(); app.bulk_highlight = false; }
        KeyCode::Char('u') => { app.jump_up(); app.bulk_highlight = false; }

        KeyCode::Char('1') => { app.filter = Filter::All; app.state.select(Some(0)); app.bulk_highlight = true; app.app_filter_query.clear(); }
        KeyCode::Char('2') => { app.filter = Filter::Skhd; app.state.select(Some(0)); app.bulk_highlight = true; app.app_filter_query.clear(); }
        KeyCode::Char('3') => {
            if app.filter == Filter::Karabiner {
                app.is_filtering_app = !app.is_filtering_app;
                if !app.is_filtering_app { app.app_filter_query.clear(); }
            } else {
                app.filter = Filter::Karabiner;
                app.is_filtering_app = true;
                app.app_filter_query.clear();
            }
            app.state.select(Some(0));
            app.bulk_highlight = true;
        }
        KeyCode::Char('4') => { app.filter = Filter::System; app.state.select(Some(0)); app.bulk_highlight = true; app.app_filter_query.clear(); }

        _ => {}
    }
    Ok(false)
}

// --- Visual Layout Editor ---
fn handle_visual_edit_input(app: &mut App, key: event::KeyEvent) {
    if app.keyboards.is_empty() {
        app.edit_mode = EditMode::None;
        return;
    }
    let idx = app.selected_keyboard_idx;
    let layout = &app.keyboards[idx].layout;
    if layout.is_empty() {
        app.edit_mode = EditMode::None;
        return;
    }

    let code = match key.code {
        KeyCode::Char(c) => KeyCode::Char(translate_char(c)),
        _ => key.code,
    };

    match code {
        // Navigation
        KeyCode::Left | KeyCode::Char('h') => {
            if app.edit_selected_col > 0 {
                app.edit_selected_col -= 1;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            let row_len = layout[app.edit_selected_row].len();
            if app.edit_selected_col < row_len.saturating_sub(1) {
                app.edit_selected_col += 1;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.edit_selected_row > 0 {
                app.edit_selected_row -= 1;
                let row_len = layout[app.edit_selected_row].len();
                if app.edit_selected_col >= row_len {
                    app.edit_selected_col = row_len.saturating_sub(1);
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.edit_selected_row < layout.len() - 1 {
                app.edit_selected_row += 1;
                let row_len = layout[app.edit_selected_row].len();
                if app.edit_selected_col >= row_len {
                    app.edit_selected_col = row_len.saturating_sub(1);
                }
            }
        }
        // Increase width
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let row = app.edit_selected_row;
            let col = app.edit_selected_col;
            if row < app.keyboards[idx].layout.len() && col < app.keyboards[idx].layout[row].len() {
                app.keyboards[idx].layout[row][col].width += 1;
            }
        }
        // Decrease width
        KeyCode::Char('-') => {
            let row = app.edit_selected_row;
            let col = app.edit_selected_col;
            if row < app.keyboards[idx].layout.len() && col < app.keyboards[idx].layout[row].len() {
                let w = &mut app.keyboards[idx].layout[row][col].width;
                if *w > 2 {
                    *w -= 1;
                }
            }
        }
        // Insert key after current
        KeyCode::Char('i') => {
            let row = app.edit_selected_row;
            let col = app.edit_selected_col;
            if row < app.keyboards[idx].layout.len() {
                let new_key = KeyDef {
                    display: "new".to_string(),
                    id: "new".to_string(),
                    width: 6,
                };
                let insert_pos = (col + 1).min(app.keyboards[idx].layout[row].len());
                app.keyboards[idx].layout[row].insert(insert_pos, new_key);
                app.edit_selected_col = insert_pos;
            }
        }
        // Delete key
        KeyCode::Char('x') => {
            let row = app.edit_selected_row;
            let col = app.edit_selected_col;
            if row < app.keyboards[idx].layout.len() && !app.keyboards[idx].layout[row].is_empty() {
                app.keyboards[idx].layout[row].remove(col);
                if app.keyboards[idx].layout[row].is_empty() {
                    // Remove empty row
                    app.keyboards[idx].layout.remove(row);
                    if app.edit_selected_row >= app.keyboards[idx].layout.len() && !app.keyboards[idx].layout.is_empty() {
                        app.edit_selected_row = app.keyboards[idx].layout.len() - 1;
                    }
                    app.edit_selected_col = 0;
                } else if app.edit_selected_col >= app.keyboards[idx].layout[row].len() {
                    app.edit_selected_col = app.keyboards[idx].layout[row].len() - 1;
                }
            }
        }
        // Add new row
        KeyCode::Char('a') => {
            let new_row = vec![KeyDef {
                display: "new".to_string(),
                id: "new".to_string(),
                width: 6,
            }];
            app.keyboards[idx].layout.push(new_row);
            app.edit_selected_row = app.keyboards[idx].layout.len() - 1;
            app.edit_selected_col = 0;
        }
        // Edit key text (Enter -> KeyInput modal)
        KeyCode::Enter => {
            let row = app.edit_selected_row;
            let col = app.edit_selected_col;
            if row < app.keyboards[idx].layout.len() && col < app.keyboards[idx].layout[row].len() {
                app.edit_input_buffer = app.keyboards[idx].layout[row][col].display.clone();
                app.edit_input_field = EditField::KeyDisplay;
                app.edit_mode = EditMode::KeyInput;
            }
        }
        // Save and exit
        KeyCode::Esc => {
            let _ = app.save_keyboard(idx);
            app.edit_mode = EditMode::None;
            app.set_status("✅ Layout saved!");
        }
        _ => {}
    }
}

// --- Key Input Modal (Display + ID) ---
fn handle_key_input_events(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.edit_input_buffer.clear();
            app.edit_mode = EditMode::Visual;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            // Save current field and switch
            let idx = app.selected_keyboard_idx;
            let row = app.edit_selected_row;
            let col = app.edit_selected_col;
            if idx < app.keyboards.len() && row < app.keyboards[idx].layout.len() && col < app.keyboards[idx].layout[row].len() {
                match app.edit_input_field {
                    EditField::KeyDisplay => {
                        if !app.edit_input_buffer.is_empty() {
                            app.keyboards[idx].layout[row][col].display = app.edit_input_buffer.clone();
                        }
                        app.edit_input_buffer = app.keyboards[idx].layout[row][col].id.clone();
                        app.edit_input_field = EditField::KeyId;
                    }
                    EditField::KeyId => {
                        if !app.edit_input_buffer.is_empty() {
                            app.keyboards[idx].layout[row][col].id = app.edit_input_buffer.clone();
                        }
                        app.edit_input_buffer = app.keyboards[idx].layout[row][col].display.clone();
                        app.edit_input_field = EditField::KeyDisplay;
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Enter => {
            let idx = app.selected_keyboard_idx;
            let row = app.edit_selected_row;
            let col = app.edit_selected_col;
            if idx < app.keyboards.len() && row < app.keyboards[idx].layout.len() && col < app.keyboards[idx].layout[row].len() {
                match app.edit_input_field {
                    EditField::KeyDisplay => {
                        if !app.edit_input_buffer.is_empty() {
                            app.keyboards[idx].layout[row][col].display = app.edit_input_buffer.clone();
                        }
                        // Switch to ID field
                        app.edit_input_buffer = app.keyboards[idx].layout[row][col].id.clone();
                        app.edit_input_field = EditField::KeyId;
                    }
                    EditField::KeyId => {
                        if !app.edit_input_buffer.is_empty() {
                            app.keyboards[idx].layout[row][col].id = app.edit_input_buffer.clone();
                        }
                        // Done, back to visual
                        app.edit_input_buffer.clear();
                        app.edit_mode = EditMode::Visual;
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Backspace => {
            app.edit_input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.edit_input_buffer.push(c);
        }
        _ => {}
    }
}

// --- Keyboard Name Input Modal ---
fn handle_keyboard_name_input_events(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.edit_input_buffer.clear();
            app.edit_mode = EditMode::None;
        }
        KeyCode::Enter => {
            let new_name = app.edit_input_buffer.trim().to_string();
            if new_name.is_empty() {
                app.set_status("Name cannot be empty");
                return;
            }

            if app.edit_input_field == EditField::KeyboardName {
                // Clone mode: create new keyboard with cloned layout
                let source_idx = app.keyboard_dropdown_idx.min(app.keyboards.len().saturating_sub(1));
                if source_idx < app.keyboards.len() {
                    let mut new_kb = app.keyboards[source_idx].clone();
                    new_kb.name = new_name;
                    app.keyboards.push(new_kb);
                    let new_idx = app.keyboards.len() - 1;
                    app.selected_keyboard_idx = new_idx;
                    let _ = app.save_keyboard(new_idx);
                    app.set_status("✅ New keyboard created!");
                }
            } else {
                // Rename mode
                let idx = app.keyboard_dropdown_idx.min(app.keyboards.len().saturating_sub(1));
                if idx < app.keyboards.len() {
                    let old_name = app.keyboards[idx].name.clone();
                    let _ = app.delete_keyboard_file(&old_name);
                    app.keyboards[idx].name = new_name;
                    let _ = app.save_keyboard(idx);
                    app.set_status("✅ Keyboard renamed!");
                }
            }

            app.edit_input_buffer.clear();
            app.edit_mode = EditMode::None;
        }
        KeyCode::Backspace => {
            app.edit_input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.edit_input_buffer.push(c);
        }
        _ => {}
    }
}
