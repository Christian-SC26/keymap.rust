use crate::app::{App, Filter, KeyDef, EditMode, EditField};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::HashSet;

const SPECIAL_KEYS: &[&str] = &[
    "esc", "enter", "return", "space", "tab", "backspace", "del", "pgup", "pgdn", "home", "end",
    "fn", "caps", "lshift", "rshift", "lctrl", "rctrl", "lopt", "ropt", "lcmd", "rcmd", "up",
    "down", "left", "right"
];

// --- Вспомогательные функции ---
fn wrap_text(text: &str, width: usize) -> String {
    let mut result = String::new();
    for line in text.lines() {
        let mut current_line_width = 0;
        for word in line.split_whitespace() {
            let word_len = word.chars().count();
            if current_line_width == 0 {
                result.push_str(word);
                current_line_width = word_len;
            } else if current_line_width + 1 + word_len > width {
                result.push('\n');
                result.push_str(word);
                current_line_width = word_len;
            } else {
                result.push(' ');
                result.push_str(word);
                current_line_width += 1 + word_len;
            }
        }
        result.push('\n');
    }
    result.trim_end().to_string()
}

fn is_key_matched(ak_lower: &str, key_id: &str, display: &str) -> bool {
    // 1. Точное совпадение (без учета регистра)
    if ak_lower == key_id {
        return true;
    }
    
    // Специальная обработка знаков препинания и символов
    let disp_lower = display.to_lowercase();
    if ak_lower == disp_lower {
        return true;
    }

    // Обработка синонимов для знаков
    match ak_lower {
        "," | "comma" if key_id == "comma" || disp_lower == "," => return true,
        "." | "period" if key_id == "period" || disp_lower == "." => return true,
        "/" | "slash" if key_id == "slash" || disp_lower == "/" => return true,
        ";" | "semicolon" if key_id == "semicolon" || disp_lower == ";" => return true,
        "'" | "quote" if key_id == "quote" || disp_lower == "'" => return true,
        "[" | "open_bracket" if key_id == "open_bracket" || disp_lower == "[" => return true,
        "]" | "close_bracket" if key_id == "close_bracket" || disp_lower == "]" => return true,
        "\\" | "backslash" if key_id == "backslash" || disp_lower == "\\" => return true,
        "~" | "grave_accent_and_tilde" if key_id == "grave_accent_and_tilde" || disp_lower == "~" || disp_lower == "ˋ" => return true,
        "-" | "hyphen" if key_id == "hyphen" || disp_lower == "-" => return true,
        "=" | "equal_sign" if key_id == "equal_sign" || disp_lower == "=" => return true,
        _ => {}
    }

    // Специальная обработка Hyper
    if ak_lower == "hyper" {
        let is_modifier = key_id.contains("cmd") || key_id.contains("opt") || 
                         key_id.contains("ctrl") || key_id.contains("shift");
        if is_modifier { return true; }
    }

    // Синонимы для стрелок
    match ak_lower {
        "up" if key_id == "up" || display == "↑" => return true,
        "down" if key_id == "down" || display == "↓" => return true,
        "left" if key_id == "left" || display == "←" => return true,
        "right" if key_id == "right" || display == "→" => return true,
        _ => {}
    }

    // 2. Модификаторы
    let has_ctrl = ak_lower.contains("ctrl") || ak_lower.contains("control");
    let has_opt = ak_lower.contains("opt") || ak_lower.contains("option") || ak_lower.contains("alt");
    let has_cmd = ak_lower.contains("cmd") || ak_lower.contains("command");
    let has_shift = ak_lower.contains("shift");

    let is_left_only = ak_lower.starts_with('l') && (has_ctrl || has_opt || has_cmd || has_shift);
    let is_right_only = ak_lower.starts_with('r') && (has_ctrl || has_opt || has_cmd || has_shift);

    if has_ctrl && key_id.contains("ctrl") {
        if is_left_only && !key_id.contains('l') { return false; }
        if is_right_only && !key_id.contains('r') { return false; }
        return true;
    }
    if has_opt && key_id.contains("opt") {
        if is_left_only && !key_id.contains('l') { return false; }
        if is_right_only && !key_id.contains('r') { return false; }
        return true;
    }
    if has_cmd && key_id.contains("cmd") {
        if is_left_only && !key_id.contains('l') { return false; }
        if is_right_only && !key_id.contains('r') { return false; }
        return true;
    }
    if has_shift && key_id.contains("shift") {
        if is_left_only && !key_id.contains('l') { return false; }
        if is_right_only && !key_id.contains('r') { return false; }
        return true;
    }

    false
}

fn get_keyboard_layout(app: &App) -> Vec<Vec<KeyDef>> {
    if app.keyboards.is_empty() {
        return vec![];
    }
    let idx = app.selected_keyboard_idx.min(app.keyboards.len() - 1);
    app.keyboards[idx].layout.clone()
}

fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let raw_area = f.area();
    if raw_area.width < 100 && app.show_overview {
        let warning_text = Line::from(vec![
            Span::styled("⚠️  Terminal too narrow! ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Overview hidden. Expand terminal window to width >= 100.", Style::default().fg(Color::DarkGray)),
        ]);
        f.render_widget(Paragraph::new(warning_text), raw_area);
        return;
    }

    if app.show_overview {
        render_overview(f, app);
        return;
    }

    // Apply unified outer margins from the terminal screen edges (vertical: 0 to remove top/bottom gaps)
    let area = raw_area.inner(Margin { vertical: 0, horizontal: 2 });
    let show_keyboard = area.width >= 100;

    let (_, keyboard_area, table_area, footer_area, info_area) = if show_keyboard {
        let r = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(13),    // keyboard layout
                Constraint::Length(1),     // spacer between keyboard and table (5)
                Constraint::Min(10),       // shortcuts table
                Constraint::Length(1),     // spacer between table and footer (6)
                Constraint::Length(1),     // footer
                Constraint::Length(1),     // spacer between footer and info bar (7)
                Constraint::Length(1),     // info bar
            ])
            .split(area);
        (r.clone(), Some(r[0]), r[2], r[4], r[6])
    } else {
        let r = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),       // shortcuts table
                Constraint::Length(1),     // spacer between table and footer (6)
                Constraint::Length(1),     // footer
                Constraint::Length(1),     // spacer between footer and info bar (7)
                Constraint::Length(1),     // info bar
            ])
            .split(area);
        (r.clone(), None, r[0], r[2], r[4])
    };

    let filtered_len;
    let selected_idx = app.state.selected().unwrap_or(0);

    {
        let filtered = app.filtered_items();
        filtered_len = filtered.len();
        
        let (active_keys, active_source) = if app.is_filtering_modifier {
            let mut all_keys: Vec<String> = filtered.iter().flat_map(|i| i.keys.iter().map(|k| k.to_lowercase())).collect();
            for m in &app.active_modifiers {
                all_keys.push(m.clone());
            }
            (all_keys, "chord_mode")
        } else if app.bulk_highlight {
            let all_keys: Vec<String> = filtered.iter().flat_map(|i| i.keys.iter().map(|k| k.to_lowercase())).collect();
            (all_keys, app.filter.as_str())
        } else if app.is_filtering_key && app.key_filter.is_some() {
            (vec![app.key_filter.unwrap().to_string().to_lowercase()], "key_mode")
        } else if let Some(item) = filtered.get(selected_idx) {
            (item.keys.iter().map(|k| k.to_lowercase()).collect(), item.source.as_str())
        } else {
            (vec![], "")
        };

        if let Some(kbd_area) = keyboard_area {
            if (app.filter == Filter::Karabiner || app.filter == Filter::Skhd) && area.width >= 125 {
                let row_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(99),  // Keyboard layout width
                        Constraint::Min(20),     // Programs list sidebar
                    ])
                    .split(kbd_area);

                draw_keyboard(f, row_chunks[0], &active_keys, &[], active_source, app);
                draw_programs_sidebar(f, row_chunks[1], app, selected_idx);
            } else {
                draw_keyboard(f, kbd_area, &active_keys, &[], active_source, app);
            }
        }

        // Вычисление доступной ширины для колонок
        let total_width = area.width as usize;
        let fixed_width = 12 + 18 + 12 + 2; // Source (12) + Trigger (18) + Spacings (3*4) + Borders (2)
        let action_width = (total_width as f32 * 0.30) as usize; // 30% для Action
        let desc_column_width = total_width.saturating_sub(fixed_width + action_width).max(20);

        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let rows: Vec<Row> = filtered
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = Some(i) == app.state.selected();
                
                // Врапим текст для ВСЕХ строк
                let desc_text = wrap_text(&item.desc, desc_column_width);
                let action_text = wrap_text(&item.action, action_width);
                
                // Считаем максимальное количество строк между Action и Description
                let desc_lines = desc_text.lines().count();
                let action_lines = action_text.lines().count();
                let mut height = desc_lines.max(action_lines).max(1) as u16;
                
                if is_selected {
                    height = height.max(2); // Даем чуть больше визуального пространства выделенной строке
                }

                let display_source = item.source.split_whitespace()
                    .filter(|tag| *tag == "karabiner" || *tag == "skhd" || *tag == "system")
                    .collect::<Vec<_>>();
                let mut unique_sources = display_source.clone(); 
                unique_sources.sort(); 
                unique_sources.dedup();
                let source_str = unique_sources.join(", ");

                let trigger_str = if item.keys.is_empty() { "-".to_string() } else { item.keys.join(" + ") };

                Row::new(vec![
                    Cell::from(format!(" {}", source_str)),
                    Cell::from(trigger_str),
                    Cell::from(Text::from(action_text)),
                    Cell::from(Text::from(desc_text)),
                ])
                .height(height)
            })
            .collect();

        let table_border_style = if app.is_searching {
            Style::default().fg(Color::Yellow)
        } else if app.is_filtering_app {
            Style::default().fg(Color::Blue)
        } else if app.is_filtering_key {
            Style::default().fg(Color::Green)
        } else if app.is_filtering_modifier {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let table_title = if !show_keyboard {
            format!(" Shortcuts (Total: {}) [⚠️ Terminal too narrow! Keyboard hidden] ", filtered_len)
        } else {
            format!(" Shortcuts (Total: {}) ", filtered_len)
        };

        let table = Table::new(rows, [Constraint::Length(12), Constraint::Length(18), Constraint::Percentage(30), Constraint::Min(40)])
            .column_spacing(4)
            .header(Row::new(vec![" Source", "Trigger", "Action", "Description"]).style(Style::default().fg(Color::DarkGray)))
            .block(Block::default().borders(Borders::ALL).border_style(table_border_style).title(table_title))
            .row_highlight_style(selected_style);

        f.render_stateful_widget(table, table_area, &mut app.state);
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("░"))
        .thumb_symbol("┃");

    let mut scrollbar_state = ScrollbarState::new(filtered_len)
        .position(selected_idx);

    f.render_stateful_widget(
        scrollbar,
        table_area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    let footer_text = Line::from(vec![
        Span::raw(" "),
        Span::styled("[/]", Style::default().fg(Color::Cyan)), Span::raw(" search  |  "),
        Span::styled("[space]", Style::default().fg(Color::Cyan)), Span::raw(" key-mode  |  "),
        Span::styled("[m]", Style::default().fg(Color::Cyan)), Span::raw(" chord-mode  |  "),
        Span::styled("[?]", Style::default().fg(Color::Cyan)), Span::raw(" help"),
    ]);
    f.render_widget(Paragraph::new(footer_text), footer_area);

    let search_style = if app.is_searching { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
    let app_filter_style = if app.is_filtering_app { Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
    let key_mode_style = if app.is_filtering_key { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
    let chord_mode_style = if app.is_filtering_modifier { Style::default().fg(Color::Red).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };

    let mut info_line = vec![Span::styled(format!(" Search: {}", app.search_query), search_style)];
    if app.filter == Filter::Karabiner {
        info_line.push(Span::raw("  |  "));
        info_line.push(Span::styled(format!(" App Filter (3): {}", app.app_filter_query), app_filter_style));
    }
    info_line.push(Span::raw("  |  "));
    info_line.push(Span::styled(format!(" Key Mode (Space): {}", app.key_filter.unwrap_or(' ')), key_mode_style));
    
    let mut mods_list: Vec<String> = app.active_modifiers.iter().cloned().collect();
    mods_list.sort();
    let mods_display = if mods_list.is_empty() { "None (c:cmd, o:opt, t:ctrl, s:shift, h:hyper, n:meh)".to_string() } else { mods_list.join("+") };
    info_line.push(Span::raw("  |  "));
    info_line.push(Span::styled(format!(" Chord Mode (m): {}", mods_display), chord_mode_style));

    if let Some((ref msg, instant)) = app.status_message {
        if instant.elapsed().as_secs() < 3 {
            info_line.push(Span::raw("  |  "));
            info_line.push(Span::styled(format!("ℹ️  {}", msg), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
        }
    }

    f.render_widget(Paragraph::new(Line::from(info_line)), info_area);

    if app.show_help {
        let area = centered_rect(75, 82, f.area());
        f.render_widget(Clear, area);
        let help_text = vec![
            Line::from(vec![Span::styled(" Keyboard Shortcuts ", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled(" [q]      ", Style::default().fg(Color::Cyan)), Span::raw("Quit Application")]),
            Line::from(vec![Span::styled(" [?]      ", Style::default().fg(Color::Cyan)), Span::raw("Toggle Help Menu")]),
            Line::from(vec![Span::styled(" [/]      ", Style::default().fg(Color::Cyan)), Span::raw("Enter Search Mode")]),
            Line::from(vec![Span::styled(" [space]  ", Style::default().fg(Color::Cyan)), Span::raw("Single Key Filter Mode")]),
            Line::from(vec![Span::styled(" [m]      ", Style::default().fg(Color::Cyan)), Span::raw("Chord Filter Mode (c/o/t/s, h:hyper, n:meh)")]),
            Line::from(vec![Span::styled(" [o]      ", Style::default().fg(Color::Cyan)), Span::raw("Toggle Multi-Keyboard Overview")]),
            Line::from(vec![Span::styled(" [Shift+K]", Style::default().fg(Color::Cyan)), Span::raw("Select Keyboard Model (Keychron / MBP)")]),
            Line::from(""),
            Line::from(vec![Span::styled(" [p]      ", Style::default().fg(Color::Cyan)), Span::raw("Parse Configs")]),
            Line::from(vec![Span::styled(" [r]      ", Style::default().fg(Color::Cyan)), Span::raw("Reload JSON")]),
            Line::from(vec![Span::styled(" [s]      ", Style::default().fg(Color::Cyan)), Span::raw("Sort Shortcuts by Description")]),
            Line::from(""),
            Line::from(vec![Span::styled(" [j/k]    ", Style::default().fg(Color::Cyan)), Span::raw("Navigate List Up/Down")]),
            Line::from(vec![Span::styled(" [u/d]    ", Style::default().fg(Color::Cyan)), Span::raw("Page Up/Down")]),
            Line::from(""),
            Line::from(vec![Span::styled(" Filters ", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled(" [1]      ", Style::default().fg(Color::Cyan)), Span::raw("All")]),
            Line::from(vec![Span::styled(" [2]      ", Style::default().fg(Color::Magenta)), Span::raw("skhd")]),
            Line::from(vec![Span::styled(" [3]      ", Style::default().fg(Color::Blue)), Span::raw("Karabiner (Type app name to filter)")]),
            Line::from(vec![Span::styled(" [4]      ", Style::default().fg(Color::Yellow)), Span::raw("System")]),
            Line::from(""),
            Line::from(vec![Span::styled(" Overview (o): ", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::raw("  Highlights keys used per source.")]),
            Line::from(vec![Span::raw("  "), Span::styled("Bright Green", Style::default().fg(Color::Rgb(0, 255, 127))), Span::raw(" keys are free in ALL sources.")]),
            Line::from(""),
            Line::from(vec![Span::styled(" Keyboard Lighting Legend ", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("● ", Style::default().fg(Color::Magenta)), Span::raw("skhd active  |  "),
                Span::styled("● ", Style::default().fg(Color::Cyan)), Span::raw("Karabiner active  |  "),
                Span::styled("● ", Style::default().fg(Color::Yellow)), Span::raw("System active  |  "),
                Span::styled("● ", Style::default().fg(Color::LightBlue)), Span::raw("Key-mode match"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("● ", Style::default().fg(Color::Red)), Span::raw("Chord-mode active  |  "),
                Span::styled("● ", Style::default().fg(Color::Rgb(255, 165, 0))), Span::raw("Special Escape [esc]  |  "),
                Span::styled("● ", Style::default().fg(Color::Rgb(0, 255, 127))), Span::raw("Free key (unused)"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(" Program Rule Statuses (Sidebar) ", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(vec![
                Span::raw("  "), Span::styled("● [Active] (Green)", Style::default().fg(Color::Green)), 
                Span::raw(" - Karabiner active/enabled shortcut rule"),
            ]),
            Line::from(vec![
                Span::raw("  "), Span::styled("○ [Disabled] (Red/Dim)", Style::default().fg(Color::Red)), 
                Span::raw(" - Karabiner disabled shortcut exception rule"),
            ]),
            Line::from(vec![
                Span::raw("  "), Span::styled("●/○ [Active/Disabled] (Yellow)", Style::default().fg(Color::Yellow)), 
                Span::raw(" - Karabiner mixed rule state (both enabled/disabled rules exist)"),
            ]),
            Line::from(vec![
                Span::raw("  "), Span::styled("● AppName (Theme Color)", Style::default().fg(Color::Magenta)), 
                Span::raw(" - skhd scoped rule application"),
            ]),
            Line::from(vec![
                Span::styled("    AppName (White)", Style::default().fg(Color::White)), 
                Span::raw(" - Program is in list, but selected shortcut is inactive for it"),
            ]),
        ];
        let help_block = Block::default().title(" Help ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan));
        f.render_widget(Paragraph::new(help_text).block(help_block), area);
    }

    // Keyboard selection dropdown overlay
    if app.show_keyboard_dropdown {
        let kb_count = app.keyboards.len() as u16;
        let height = kb_count.saturating_add(5).max(6); // extra space for hints
        let area = centered_rect_fixed(50, height, f.area());
        f.render_widget(Clear, area);

        let mut dropdown_lines = vec![];
        for (idx, kb) in app.keyboards.iter().enumerate() {
            let is_selected = idx == app.keyboard_dropdown_idx;
            let is_active = idx == app.selected_keyboard_idx;
            
            let mut spans = vec![];
            if is_selected {
                spans.push(Span::styled(" ▶ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
                spans.push(Span::styled(&kb.name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
            } else {
                spans.push(Span::raw("   "));
                spans.push(Span::styled(&kb.name, Style::default().fg(Color::White)));
            }
            
            if is_active {
                spans.push(Span::styled(" [Active]", Style::default().fg(Color::Green).add_modifier(Modifier::DIM)));
            }
            
            dropdown_lines.push(Line::from(spans));
        }

        // Подсказки управления
        dropdown_lines.push(Line::from(""));
        dropdown_lines.push(Line::from(vec![
            Span::styled(" [a]", Style::default().fg(Color::Green)), Span::raw(" clone  "),
            Span::styled("[d]", Style::default().fg(Color::Red)), Span::raw(" delete  "),
            Span::styled("[e]", Style::default().fg(Color::Yellow)), Span::raw(" edit  "),
            Span::styled("[r]", Style::default().fg(Color::Blue)), Span::raw(" rename"),
        ]));

        let block = Block::default()
            .title(Span::styled(" Keyboards (Shift+K) ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
            
        f.render_widget(Paragraph::new(dropdown_lines).block(block), area);
    }

    // Modal: Keyboard Name Input
    if app.edit_mode == EditMode::KeyboardNameInput {
        let area = centered_rect_fixed(50, 5, f.area());
        f.render_widget(Clear, area);

        let title = match app.edit_input_field {
            EditField::KeyboardName => " Enter Keyboard Name ",
            _ => " Rename Keyboard ",
        };

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(&app.edit_input_buffer, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled("▏", Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK)),
            ]),
            Line::from(""),
        ];

        let block = Block::default()
            .title(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    // Modal: Key Input (Display + ID)
    if app.edit_mode == EditMode::KeyInput {
        let area = centered_rect_fixed(50, 8, f.area());
        f.render_widget(Clear, area);

        let display_style = if app.edit_input_field == EditField::KeyDisplay {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let id_style = if app.edit_input_field == EditField::KeyId {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Get current key values
        let (current_display, current_id) = if app.edit_selected_row < app.keyboards[app.selected_keyboard_idx].layout.len() {
            let row = &app.keyboards[app.selected_keyboard_idx].layout[app.edit_selected_row];
            if app.edit_selected_col < row.len() {
                (row[app.edit_selected_col].display.clone(), row[app.edit_selected_col].id.clone())
            } else {
                ("?".to_string(), "?".to_string())
            }
        } else {
            ("?".to_string(), "?".to_string())
        };

        let display_val = if app.edit_input_field == EditField::KeyDisplay {
            format!("{}▏", app.edit_input_buffer)
        } else {
            current_display
        };
        let id_val = if app.edit_input_field == EditField::KeyId {
            format!("{}▏", app.edit_input_buffer)
        } else {
            current_id
        };

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Display: ", display_style),
                Span::styled(&display_val, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Key ID:  ", id_style),
                Span::styled(&id_val, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [Tab]", Style::default().fg(Color::Cyan)),
                Span::raw(" switch field  "),
                Span::styled("[Enter]", Style::default().fg(Color::Green)),
                Span::raw(" save  "),
                Span::styled("[Esc]", Style::default().fg(Color::Red)),
                Span::raw(" cancel"),
            ]),
        ];

        let block = Block::default()
            .title(Span::styled(" Edit Key ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        f.render_widget(Paragraph::new(lines).block(block), area);
    }
}

fn get_free_keys(app: &App) -> Vec<String> {
    let all_keys: Vec<String> = app.items.iter().flat_map(|i| i.keys.iter().map(|k| k.to_lowercase())).collect();
    let is_hyper = all_keys.iter().any(|k| k == "hyper");

    let mut free = Vec::new();
    let layout = get_keyboard_layout(app);
    for row in &layout {
        for keydef in row {
            let is_special = SPECIAL_KEYS.contains(&keydef.id.as_str());
            if is_special { continue; }

            let is_used = all_keys.iter().any(|k| is_key_matched(k, &keydef.id, &keydef.display)) || 
                          (is_hyper && (keydef.id.contains("cmd") || keydef.id.contains("opt") || keydef.id.contains("ctrl") || keydef.id.contains("shift")));
            
            if !is_used {
                free.push(keydef.id.to_string());
            }
        }
    }
    free
}

fn render_overview(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let mut sources = HashSet::new();
    for item in &app.items {
        for tag in item.source.split_whitespace() {
            if tag == "karabiner" || tag == "skhd" || tag == "system" { sources.insert(tag.to_string()); }
        }
    }
    let mut sources_list: Vec<_> = sources.into_iter().collect();
    sources_list.sort();

    let free_keys = get_free_keys(app);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(sources_list.iter().map(|_| Constraint::Length(18)).collect::<Vec<_>>())
        .split(area);

    for (i, source) in sources_list.iter().enumerate() {
        if i >= chunks.len() { break; }
        let source_keys: Vec<String> = app.items.iter()
            .filter(|item| item.source.contains(source))
            .flat_map(|item| item.keys.iter().map(|k| k.to_lowercase()))
            .collect();
            
        if area.width >= 135 {
            let row_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(99),  // Keyboard layout width
                    Constraint::Min(20),     // Stats and diagnostic info sidebar
                ])
                .split(chunks[i]);

            draw_keyboard(f, row_chunks[0], &source_keys, &free_keys, source, app);
            draw_diagnostics_sidebar(f, row_chunks[1], source, app);
        } else {
            draw_keyboard(f, chunks[i], &source_keys, &free_keys, source, app);
        }
    }
}

fn draw_diagnostics_sidebar(f: &mut Frame, area: Rect, source: &str, app: &App) {
    let analysis = app.analyze_source(source);
    
    // Choose dynamic color matching the source type
    let theme_color = match source.to_lowercase().as_str() {
        s if s.contains("skhd") => Color::Magenta,
        s if s.contains("karabiner") => Color::Cyan,
        s if s.contains("system") => Color::Yellow,
        _ => Color::White,
    };

    let title = format!(" {} DIAGNOSTICS & STATS ", source.to_uppercase());
    
    let mut text = vec![];
    
    // 1. Config Path and Last Modified Time
    text.push(Line::from(vec![
        Span::styled("● Config File:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(&analysis.config_path, Style::default().fg(Color::White)),
    ]));
    
    text.push(Line::from(vec![
        Span::styled("● Last Update:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(&analysis.last_modified, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]));
    
    text.push(Line::from(""));

    // 2. Metrics
    text.push(Line::from(vec![
        Span::styled("● Shortcuts:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(analysis.total_shortcuts.to_string(), Style::default().fg(theme_color).add_modifier(Modifier::BOLD)),
    ]));
    
    text.push(Line::from(vec![
        Span::styled("● Top Modifier: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&analysis.top_modifier, Style::default().fg(Color::White)),
    ]));

    text.push(Line::from(""));
    text.push(Line::from(Span::styled("-".repeat((area.width as usize).saturating_sub(4)), Style::default().fg(Color::DarkGray))));
    text.push(Line::from(""));

    // 3. Conflict Detections
    if analysis.conflicts.is_empty() {
        text.push(Line::from(vec![
            Span::styled("✅ No Cross-Source Conflicts Detected", Style::default().fg(Color::Rgb(0, 255, 127)).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        text.push(Line::from(vec![
            Span::styled("⚠️ DETECTED KEYBIND CONFLICTS:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]));
        text.push(Line::from(""));
        
        // List up to 4 conflicts beautifully so they fit in the 18 height layout
        let limit = analysis.conflicts.len().min(4);
        for i in 0..limit {
            let (sig, other_src, other_action) = &analysis.conflicts[i];
            
            // Clean action description (shorten if too long)
            let action_short = if other_action.chars().count() > 25 {
                format!("{}...", other_action.chars().take(22).collect::<String>())
            } else {
                other_action.clone()
            };
            
            text.push(Line::from(vec![
                Span::styled(format!("  • {}", sig), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" (also in {})", other_src.to_uppercase()), Style::default().fg(Color::DarkGray)),
            ]));
            text.push(Line::from(vec![
                Span::styled(format!("    ↳ {}", action_short), Style::default().fg(Color::White)),
            ]));
        }
        
        if analysis.conflicts.len() > 4 {
            text.push(Line::from(vec![
                Span::styled(format!("  ...and {} more conflicts", analysis.conflicts.len() - 4), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(title, Style::default().fg(theme_color).add_modifier(Modifier::BOLD)));

    // Fit content inside the block
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_keyboard(f: &mut Frame, area: Rect, active_keys: &[String], free_keys: &[String], source: &str, app: &App) {
    let active_color = match source.to_lowercase().as_str() {
        s if s.contains("skhd") || s.contains("sk") => Color::Magenta,
        s if s.contains("karabiner") || s.contains("ke") => Color::Cyan,
        s if s.contains("xc") => Color::Green,
        s if s.contains("system") || s.contains("sy") => Color::Yellow,
        "key_mode" => Color::LightBlue,
        "chord_mode" => Color::Red,
        _ => Color::White,
    };
    let free_color = Color::Rgb(0, 255, 127);
    let is_editing = app.edit_mode == EditMode::Visual || app.edit_mode == EditMode::KeyInput;

    // Fixed left padding to match shortcuts list indentation inside the block
    let padding = 1;
    let pad_str = " ".repeat(padding);
    let row_separator = format!("{}{}", pad_str, "-".repeat(97));

    let mut text_lines = vec![];
    let is_hyper = active_keys.iter().any(|k| k == "hyper");

    let layout = get_keyboard_layout(app);

    for (row_idx, row) in layout.iter().enumerate() {
        let mut spans = vec![];
        if padding > 0 {
            spans.push(Span::raw(pad_str.clone()));
        }

        let row_info: Vec<(bool, bool)> = row.iter().map(|keydef| {
            let is_active = active_keys.iter().any(|k| is_key_matched(k, &keydef.id, &keydef.display)) || 
                           (is_hyper && (keydef.id.contains("cmd") || keydef.id.contains("opt") || keydef.id.contains("ctrl") || keydef.id.contains("shift")));
            let is_free = free_keys.iter().any(|k| k == &keydef.id);
            (is_active, is_free)
        }).collect();

        for (i, keydef) in row.iter().enumerate() {
            let is_esc = keydef.id == "esc";
            let (is_active, is_free) = row_info[i];
            let is_cursor = is_editing && row_idx == app.edit_selected_row && i == app.edit_selected_col;
            let prev_active = if i > 0 { row_info[i-1].0 } else { false };
            let slash_active = is_active || prev_active;
            let slash_style = if slash_active { Style::default().fg(active_color).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
            spans.push(Span::styled("/", slash_style));

            let (_color, style) = if is_cursor {
                (Color::Yellow, Style::default().fg(Color::Yellow).add_modifier(Modifier::REVERSED | Modifier::BOLD))
            } else if is_esc {
                (Color::Rgb(255, 165, 0), Style::default().fg(Color::Rgb(255, 165, 0)).add_modifier(Modifier::BOLD))
            } else if is_active {
                (active_color, Style::default().fg(active_color).add_modifier(Modifier::BOLD))
            } else if is_free {
                (free_color, Style::default().fg(free_color).add_modifier(Modifier::BOLD))
            } else {
                (Color::DarkGray, Style::default().fg(Color::DarkGray))
            };
            
            let inner_width = keydef.width - 1;
            let text_chars = keydef.display.chars().count();
            let left_pad = inner_width.saturating_sub(text_chars) / 2;
            let right_pad = inner_width.saturating_sub(text_chars) - left_pad;
            let key_text = format!("{spaces_left}{text}{spaces_right}", spaces_left = " ".repeat(left_pad), text = keydef.display, spaces_right = " ".repeat(right_pad));
            spans.push(Span::styled(key_text, style));
            if i == row.len() - 1 {
                let last_slash_style = if is_active { Style::default().fg(active_color).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
                spans.push(Span::styled("/", last_slash_style));
            }
        }
        text_lines.push(Line::from(spans));
        if row_idx < layout.len() - 1 {
            text_lines.push(Line::from(Span::styled(&row_separator, Style::default().fg(Color::DarkGray))));
        }
    }

    let kb_name = if !app.keyboards.is_empty() {
        let idx = app.selected_keyboard_idx.min(app.keyboards.len() - 1);
        app.keyboards[idx].name.as_str()
    } else {
        "No Keyboard"
    };

    let title = if is_editing {
        format!(" {} [EDIT: h/j/k/l move, +/- width, i insert, d delete, Enter edit, Esc save] ", kb_name)
    } else {
        format!(" {} - {} Layout ", kb_name, source.to_uppercase())
    };

    let border_color = if is_editing { Color::Yellow } else { Color::DarkGray };
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color)).title(title);
    f.render_widget(Paragraph::new(text_lines).block(block), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage((100 - percent_y) / 2), Constraint::Percentage(percent_y), Constraint::Percentage((100 - percent_y) / 2)]).split(r);
    Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage((100 - percent_x) / 2), Constraint::Percentage(percent_x), Constraint::Percentage((100 - percent_x) / 2)]).split(popup_layout[1])[1]
}

fn get_app_name_for_slug(slug: &str, aliases: &std::collections::HashMap<String, String>) -> String {
    for (k, v) in aliases {
        if v.to_lowercase() == slug.to_lowercase() {
            let mut chars = k.chars();
            return match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            };
        }
    }
    let mut chars = slug.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn is_valid_app_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Must start with an alphabetic character
    if let Some(first_char) = trimmed.chars().next() {
        if !first_char.is_alphabetic() {
            return false;
        }
    }
    // Shouldn't contain shell commands
    if trimmed.contains("--") || trimmed.contains("::") || trimmed.contains('$') {
        return false;
    }
    true
}

fn draw_programs_sidebar(f: &mut Frame, area: Rect, app: &App, selected_idx: usize) {
    let current_filter = app.filter;
    let source_name = current_filter.as_str(); // "karabiner" or "skhd"
    let theme_color = match current_filter {
        Filter::Karabiner => Color::Cyan,
        Filter::Skhd => Color::Magenta,
        _ => Color::White,
    };

    // 1. Collect all unique programs for the current source
    let mut all_apps = std::collections::HashSet::new();
    for item in &app.items {
        if item.source.contains(source_name) {
            if current_filter == Filter::Karabiner {
                for rule_tag in item.rules.split_whitespace() {
                    if let Some(idx) = rule_tag.rfind('_') {
                        let slug = &rule_tag[..idx];
                        let app_name = get_app_name_for_slug(slug, &app.aliases);
                        if !app_name.is_empty() {
                            all_apps.insert(app_name);
                        }
                    }
                }
            } else {
                // skhd
                if item.desc.contains('|') {
                    for part in item.desc.split('|') {
                        if let Some(idx) = part.find(':') {
                            let app_name = part[..idx].trim().to_string();
                            if app_name != "*" && app_name != "Остальные" && is_valid_app_name(&app_name) {
                                all_apps.insert(app_name);
                            }
                        }
                    }
                } else if let Some(idx) = item.desc.find(':') {
                    let app_name = item.desc[..idx].trim().to_string();
                    if app_name.len() < 30 && !app_name.contains('/') && is_valid_app_name(&app_name) && app_name != "*" && app_name != "Остальные" {
                        all_apps.insert(app_name);
                    }
                }
            }
        }
    }

    let mut sorted_apps: Vec<String> = all_apps.into_iter().collect();
    sorted_apps.sort();

    // 2. Identify status for the currently selected shortcut
    let mut active_apps_status: std::collections::HashMap<String, std::collections::HashSet<String>> = std::collections::HashMap::new();
    if let Some(selected_item) = app.filtered_items().get(selected_idx) {
        if current_filter == Filter::Karabiner {
            for rule_tag in selected_item.rules.split_whitespace() {
                if let Some(idx) = rule_tag.rfind('_') {
                    let slug = &rule_tag[..idx];
                    let suffix = &rule_tag[idx+1..]; // "e" or "d"
                    let app_name = get_app_name_for_slug(slug, &app.aliases);
                    if !app_name.is_empty() {
                        active_apps_status.entry(app_name)
                            .or_insert_with(std::collections::HashSet::new)
                            .insert(suffix.to_string());
                    }
                }
            }
        } else {
            // skhd
            if selected_item.desc.contains('|') {
                for part in selected_item.desc.split('|') {
                    if let Some(idx) = part.find(':') {
                        let app_name = part[..idx].trim().to_string();
                        if app_name != "*" && app_name != "Остальные" && is_valid_app_name(&app_name) {
                            active_apps_status.entry(app_name)
                                .or_insert_with(std::collections::HashSet::new)
                                .insert("skhd_active".to_string());
                        }
                    }
                }
            } else if let Some(idx) = selected_item.desc.find(':') {
                let app_name = selected_item.desc[..idx].trim().to_string();
                if app_name.len() < 30 && !app_name.contains('/') && is_valid_app_name(&app_name) && app_name != "*" && app_name != "Остальные" {
                    active_apps_status.entry(app_name)
                        .or_insert_with(std::collections::HashSet::new)
                        .insert("skhd_active".to_string());
                }
            }
        }
    }

    // 3. Render list
    let mut list_lines = vec![];
    list_lines.push(Line::from("")); // Spacer

    if sorted_apps.is_empty() {
        list_lines.push(Line::from(vec![
            Span::styled("  No specific app rules", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
        ]));
    } else {
        for app_name in &sorted_apps {
            let line = if let Some(statuses) = active_apps_status.get(app_name) {
                if statuses.contains("e") && statuses.contains("d") {
                    Line::from(vec![
                        Span::styled("  ●/○ ", Style::default().fg(Color::Yellow)),
                        Span::styled(format!("{} [Active/Disabled]", app_name), Style::default().fg(theme_color).add_modifier(Modifier::BOLD)),
                    ])
                } else if statuses.contains("e") {
                    Line::from(vec![
                        Span::styled("  ● ", Style::default().fg(Color::Green)),
                        Span::styled(format!("{} [Active]", app_name), Style::default().fg(theme_color).add_modifier(Modifier::BOLD)),
                    ])
                } else if statuses.contains("d") {
                    Line::from(vec![
                        Span::styled("  ○ ", Style::default().fg(Color::Red)),
                        Span::styled(format!("{} [Disabled]", app_name), Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)),
                    ])
                } else {
                    // skhd_active
                    Line::from(vec![
                        Span::styled("  ● ", Style::default().fg(theme_color)),
                        Span::styled(app_name, Style::default().fg(theme_color).add_modifier(Modifier::BOLD)),
                    ])
                }
            } else {
                Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(app_name, Style::default().fg(Color::White)),
                ])
            };
            list_lines.push(line);
        }
    }

    let title = format!(" {} Programs ", source_name.to_uppercase());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme_color))
        .title(Span::styled(title, Style::default().fg(theme_color).add_modifier(Modifier::BOLD)));

    let paragraph = Paragraph::new(list_lines).block(block);
    f.render_widget(paragraph, area);
}
