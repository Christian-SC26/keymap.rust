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
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let output_path = format!("{}/.config/karabiner/shortcuts.json", home);
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
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if app.show_help {
                app.show_help = false;
                continue;
            }

            if app.is_searching {
                handle_search_input(app, key);
            } else {
                if handle_navigation_input(app, key)? {
                    return Ok(());
                }
            }
        }
    }
}

/// Транслирует символ из другой раскладки в эквивалентную клавишу QWERTY.
/// Это позволяет использовать горячие клавиши независимо от текущего языка ввода.
/// ВАЖНО: Мы не делаем обратных замен (например, q -> a), чтобы не ломать QWERTY.
fn translate_char(c: char) -> char {
    let lower_c = c.to_lowercase().next().unwrap_or(c);
    let translated = match lower_c {
        // Русская раскладка (ЙЦУКЕН)
        'й' => 'q',
        'ц' => 'w',
        'у' => 'e',
        'к' => 'r',
        'е' => 't',
        'н' => 'y',
        'г' => 'u',
        'ш' => 'i',
        'щ' => 'o',
        'з' => 'p',
        'х' => '[',
        'ъ' => ']',
        'ф' => 'a',
        'ы' => 's',
        'в' => 'd',
        'а' => 'f',
        'п' => 'g',
        'р' => 'h',
        'о' => 'j',
        'л' => 'k',
        'д' => 'l',
        'ж' => ';',
        'э' => '\'',
        'я' => 'z',
        'ч' => 'x',
        'с' => 'c',
        'м' => 'v',
        'и' => 'b',
        'т' => 'n',
        'ь' => 'm',
        'б' => ',',
        'ю' => '.',
        '.' => '/',
        ',' => '?',

        // Французская раскладка (AZERTY) - только безопасные маппинги на пустые клавиши
        'a' => 'q', // На месте Q в AZERTY стоит A (A не занята в навигации)
        'z' => 'w', // На месте W в AZERTY стоит Z (Z не занята в навигации)
        // 'q' => 'a', // НЕЛЬЗЯ: это сломает выход (Quit) для QWERTY
        // 'w' => 'z', // НЕЛЬЗЯ: это может сломать другие горячие клавиши
        'm' => ';',

        // Немецкая раскладка (QWERTZ)
        'y' => 'z', // В QWERTY 'y' не используется
        'ö' => ';',
        'ä' => '\'',
        'ü' => '[',

        _ => lower_c,
    };

    // Возвращаем в исходном регистре, если это возможно
    if c.is_uppercase() {
        translated.to_uppercase().next().unwrap_or(translated)
    } else {
        translated
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
        // Quit
        KeyCode::Char('q') => return Ok(true),

        // Reload
        KeyCode::Char('r') => {
            let _ = app.reload();
        }

        // Parse
        KeyCode::Char('p') => {
            if let Some(ref path) = app.config_path {
                let _ = parser::run_parser(path);
            }
        }

        // Sort
        KeyCode::Char('s') => {
            app.sort_shortcuts();
        }

        // Help
        KeyCode::Char('?') => {
            app.show_help = true;
        }

        // Search
        KeyCode::Char('/') => {
            app.is_searching = true;
        }

        // Clear search
        KeyCode::Esc if !app.search_query.is_empty() => {
            app.search_query.clear();
            app.state.select(Some(0));
            app.bulk_highlight = false;
        }

        // Navigation
        KeyCode::Down | KeyCode::Char('j') => {
            app.next();
            app.bulk_highlight = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.previous();
            app.bulk_highlight = false;
        }

        // Filters
        KeyCode::Char('1') => {
            app.filter = Filter::All;
            app.state.select(Some(0));
            app.bulk_highlight = true;
        }
        KeyCode::Char('2') => {
            app.filter = Filter::Skhd;
            app.state.select(Some(0));
            app.bulk_highlight = true;
        }
        KeyCode::Char('3') => {
            app.filter = Filter::Karabiner;
            app.state.select(Some(0));
            app.bulk_highlight = true;
        }
        KeyCode::Char('4') => {
            app.filter = Filter::Xcode;
            app.state.select(Some(0));
            app.bulk_highlight = true;
        }
        KeyCode::Char('5') => {
            app.filter = Filter::System;
            app.state.select(Some(0));
            app.bulk_highlight = true;
        }

        _ => {}
    }
    Ok(false)
}
