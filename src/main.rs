use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;
use std::io;

mod app;
mod parser;
mod ui;

use crate::app::{App, Filter};
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
                if handle_modifier_filter_input(app, key)? {
                    return Ok(());
                }
            } else {
                if handle_navigation_input(app, key)? {
                    return Ok(());
                }
            }
        }
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

fn handle_modifier_filter_input(app: &mut App, key: event::KeyEvent) -> io::Result<bool> {
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
        _ => handle_navigation_input(app, key),
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
    match key.code {
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
    match key.code {
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

fn handle_navigation_input(app: &mut App, key: event::KeyEvent) -> io::Result<bool> {
    let code = match key.code {
        KeyCode::Char(c) => KeyCode::Char(translate_char(c)),
        _ => key.code,
    };

    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('r') => { let _ = app.reload(); }
        KeyCode::Char('p') => { if let Some(ref path) = app.config_path { let _ = parser::run_parser(path); } }
        KeyCode::Char('s') => { app.sort_shortcuts(); }
        KeyCode::Char('?') => { app.show_help = true; }
        KeyCode::Char('/') => { app.is_searching = true; }
        KeyCode::Char('o') => { app.show_overview = !app.show_overview; }
        
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
