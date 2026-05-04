use crate::app::App;
use crate::app::Filter;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};
use std::collections::HashSet;

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

// --- Наша новая клавиатура удалена, загружается из JSON ---

pub fn ui(f: &mut Frame, app: &mut App) {
    if app.show_overview {
        render_overview(f, app);
        return;
    }

    let area = f.area();
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), Constraint::Length(15), Constraint::Min(10), Constraint::Length(1), Constraint::Length(1),
        ])
        .split(area);

    let filtered = app.filtered_items();
    let selected_idx = app.state.selected().unwrap_or(0);
    
    let (active_keys, active_source) = if app.bulk_highlight {
        let all_keys: Vec<String> = filtered.iter().flat_map(|i| i.keys.iter().map(|k| k.to_lowercase())).collect();
        (all_keys, app.filter.as_str())
    } else if app.is_filtering_key && app.key_filter.is_some() {
        (vec![app.key_filter.unwrap().to_string().to_lowercase()], "key_mode")
    } else if let Some(item) = filtered.get(selected_idx) {
        (item.keys.iter().map(|k| k.to_lowercase()).collect(), item.source.as_str())
    } else {
        (vec![], "")
    };

    draw_keyboard(f, rects[1], &active_keys, &[], active_source, app);

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
                .filter(|tag| *tag == "ke" || *tag == "sk" || *tag == "sy")
                .collect::<Vec<_>>();
            let mut unique_sources = display_source.clone(); 
            unique_sources.sort(); 
            unique_sources.dedup();
            let source_str = unique_sources.join(", ");

            let trigger_str = if item.keys.is_empty() { "-".to_string() } else { item.keys.join(" + ") };

            Row::new(vec![
                Cell::from(source_str),
                Cell::from(trigger_str),
                Cell::from(Text::from(action_text)),
                Cell::from(Text::from(desc_text)),
            ])
            .height(height)
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Length(18), Constraint::Percentage(30), Constraint::Min(40)])
        .column_spacing(4)
        .header(Row::new(vec!["Source", "Trigger", "Action", "Description"]).style(Style::default().fg(Color::DarkGray)))
        .block(Block::default().borders(Borders::ALL).title(format!(" Shortcuts (Total: {}) ", filtered.len())))
        .row_highlight_style(selected_style);

    f.render_stateful_widget(table, rects[2], &mut app.state);

    let footer_text = Line::from(vec![
        Span::styled("[q]", Style::default().fg(Color::Cyan)), Span::raw("uit  "),
        Span::styled("[p]", Style::default().fg(Color::Cyan)), Span::raw("arse  "),
        Span::styled("[s]", Style::default().fg(Color::Cyan)), Span::raw("ort  "),
        Span::styled("[r]", Style::default().fg(Color::Cyan)), Span::raw("eload  "),
        Span::styled("[/]", Style::default().fg(Color::Cyan)), Span::raw("search  "),
        Span::styled("[space]", Style::default().fg(Color::Cyan)), Span::raw(" key-mode  "),
        Span::styled("[o]", Style::default().fg(Color::Cyan)), Span::raw(" overview  "),
        Span::styled("[j/k u/d]", Style::default().fg(Color::Cyan)), Span::raw(" nav  |  Filters: "),
        Span::styled("[1]", Style::default().fg(Color::Cyan)), Span::raw(" All  "),
        Span::styled("[2]", Style::default().fg(Color::Magenta)), Span::raw(" skhd  "),
        Span::styled("[3]", Style::default().fg(Color::Blue)), Span::raw(" Karabiner+App  "),
        Span::styled("[4]", Style::default().fg(Color::Yellow)), Span::raw(" System | "),
        Span::styled("[?]", Style::default().fg(Color::Cyan)), Span::raw("help"),
    ]);
    f.render_widget(Paragraph::new(footer_text), rects[3]);

    let search_style = if app.is_searching { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
    let app_filter_style = if app.is_filtering_app { Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
    let key_mode_style = if app.is_filtering_key { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };

    let mut info_line = vec![Span::styled(format!(" Search: {}", app.search_query), search_style)];
    if app.filter == Filter::Karabiner {
        info_line.push(Span::raw("  |  "));
        info_line.push(Span::styled(format!(" App Filter (3): {}", app.app_filter_query), app_filter_style));
    }
    info_line.push(Span::raw("  |  "));
    info_line.push(Span::styled(format!(" Key Mode (Space): {}", app.key_filter.unwrap_or(' ')), key_mode_style));

    f.render_widget(Paragraph::new(Line::from(info_line)), rects[4]);

    if app.show_help {
        let area = centered_rect(60, 75, f.area());
        f.render_widget(Clear, area);
        let help_text = vec![
            Line::from(vec![Span::styled(" Keyboard Shortcuts ", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled(" [q]      ", Style::default().fg(Color::Cyan)), Span::raw("Quit Application")]),
            Line::from(vec![Span::styled(" [o]      ", Style::default().fg(Color::Cyan)), Span::raw("Toggle Multi-Keyboard Overview")]),
            Line::from(vec![Span::styled(" [?]      ", Style::default().fg(Color::Cyan)), Span::raw("Toggle Help Menu")]),
            Line::from(vec![Span::styled(" [/]      ", Style::default().fg(Color::Cyan)), Span::raw("Enter Search Mode")]),
            Line::from(vec![Span::styled(" [space]  ", Style::default().fg(Color::Cyan)), Span::raw("Single Key Filter Mode")]),
            Line::from(vec![Span::styled(" [p]      ", Style::default().fg(Color::Cyan)), Span::raw("Parse Configs")]),
            Line::from(vec![Span::styled(" [r]      ", Style::default().fg(Color::Cyan)), Span::raw("Reload JSON")]),
            Line::from(""),
            Line::from(vec![Span::styled(" [1-4]    ", Style::default().fg(Color::Cyan)), Span::raw("Apply Category Filters")]),
            Line::from(""),
            Line::from(vec![Span::styled(" Overview (o): ", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::raw("  Highlights keys used per source.")]),
            Line::from(vec![Span::raw("  "), Span::styled("Bright Green", Style::default().fg(Color::Rgb(0, 255, 127))), Span::raw(" keys are free in ALL sources.")]),
        ];
        let help_block = Block::default().title(" Help ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan));
        f.render_widget(Paragraph::new(help_text).block(help_block), area);
    }
}

fn get_free_keys(app: &App) -> Vec<String> {
    let all_keys: Vec<String> = app.items.iter().flat_map(|i| i.keys.iter().map(|k| k.to_lowercase())).collect();
    let is_hyper = all_keys.iter().any(|k| k == "hyper");

    let mut free = Vec::new();
    for row in &app.keyboard_layout {
        for keydef in row {
            let is_special = ["esc", "enter", "return", "space", "tab", "backspace", "del", "pgup", "pgdn", "home", "end", "fn", "caps", "lshift", "rshift", "lctrl", "rctrl", "lopt", "ropt", "lcmd", "rcmd", "up", "down", "left", "right"].contains(&keydef.id.as_str());
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
            if tag == "ke" || tag == "sk" || tag == "sy" { sources.insert(tag.to_string()); }
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
            
        draw_keyboard(f, chunks[i], &source_keys, &free_keys, source, app);
    }
}

fn draw_keyboard(f: &mut Frame, area: Rect, active_keys: &[String], free_keys: &[String], source: &str, app: &App) {
    let active_color = match source.to_lowercase().as_str() {
        s if s.contains("sk") => Color::Magenta,
        s if s.contains("ke") => Color::Cyan,
        s if s.contains("xc") => Color::Green,
        s if s.contains("sy") => Color::Yellow,
        "key_mode" => Color::LightBlue,
        _ => Color::White,
    };
    let free_color = Color::Rgb(0, 255, 127);

    let mut text_lines = vec![];
    let row_separator = "-".repeat(97);
    text_lines.push(Line::from(""));
    text_lines.push(Line::from(Span::styled(&row_separator, Style::default().fg(Color::DarkGray))));

    let is_hyper = active_keys.iter().any(|k| k == "hyper");

    for (row_idx, row) in app.keyboard_layout.iter().enumerate() {
        let mut spans = vec![];
        let row_info: Vec<(bool, bool)> = row.iter().map(|keydef| {
            let is_active = active_keys.iter().any(|k| is_key_matched(k, &keydef.id, &keydef.display)) || 
                           (is_hyper && (keydef.id.contains("cmd") || keydef.id.contains("opt") || keydef.id.contains("ctrl") || keydef.id.contains("shift")));
            let is_free = free_keys.iter().any(|k| k == &keydef.id);
            (is_active, is_free)
        }).collect();

        for (i, keydef) in row.iter().enumerate() {
            let is_esc = keydef.id == "esc";
            let (is_active, is_free) = row_info[i];
            let prev_active = if i > 0 { row_info[i-1].0 } else { false };
            let slash_active = is_active || prev_active;
            let slash_style = if slash_active { Style::default().fg(active_color).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
            spans.push(Span::styled("/", slash_style));
            let color = if is_esc { Color::Rgb(255, 165, 0) } else if is_active { active_color } else if is_free { free_color } else { Color::DarkGray };
            let style = if is_active || is_esc || is_free { Style::default().fg(color).add_modifier(Modifier::BOLD) } else { Style::default().fg(color) };
            
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
        if row_idx < app.keyboard_layout.len() - 1 {
            text_lines.push(Line::from(Span::styled(&row_separator, Style::default().fg(Color::DarkGray))));
        }
    }
    text_lines.push(Line::from(Span::styled(&row_separator, Style::default().fg(Color::DarkGray))));
    let title = format!(" {} Layout ", source.to_uppercase());
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(title);
    f.render_widget(Paragraph::new(text_lines).block(block), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage((100 - percent_y) / 2), Constraint::Percentage(percent_y), Constraint::Percentage((100 - percent_y) / 2)]).split(r);
    Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage((100 - percent_x) / 2), Constraint::Percentage(percent_x), Constraint::Percentage((100 - percent_x) / 2)]).split(popup_layout[1])[1]
}
