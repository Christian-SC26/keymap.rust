use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

// --- Вспомогательные функции ---
fn wrap_text(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut current_line_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        if current_line_width + word_len + 1 > width {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(word);
            current_line_width = word_len;
        } else {
            if !result.is_empty() && !result.ends_with('\n') {
                result.push(' ');
                current_line_width += 1;
            }
            result.push_str(word);
            current_line_width += word_len;
        }
    }
    result
}

fn is_key_matched(active_key: &str, key_id: &str, display: &str) -> bool {
    // 1. Точное совпадение (без учета регистра)
    if active_key.eq_ignore_ascii_case(key_id) || active_key.eq_ignore_ascii_case(display) {
        return true;
    }

    let ak_lower = active_key.to_lowercase();
    let kid_lower = key_id.to_lowercase();

    // Специальная обработка Hyper (должен подсвечивать все модификаторы)
    if ak_lower == "hyper" {
        let is_modifier = kid_lower.contains("cmd") || kid_lower.contains("opt") || 
                         kid_lower.contains("ctrl") || kid_lower.contains("shift");
        if is_modifier { return true; }
    }

    // Синонимы для стрелок
    match ak_lower.as_str() {
        "up" if key_id == "↑" || display == "↑" => return true,
        "down" if key_id == "↓" || display == "↓" => return true,
        "left" if key_id == "←" || display == "←" => return true,
        "right" if key_id == "→" || display == "→" => return true,
        _ => {}
    }

    // 2. Модификаторы (поддержка cmd, opt, ctrl, shift и их сторон l/r)
    let has_ctrl = ak_lower.contains("ctrl") || ak_lower.contains("control");
    let has_opt = ak_lower.contains("opt") || ak_lower.contains("option") || ak_lower.contains("alt");
    let has_cmd = ak_lower.contains("cmd") || ak_lower.contains("command");
    let has_shift = ak_lower.contains("shift");

    // Проверяем указание стороны (l или r) в активном ключе
    let is_left_only = ak_lower.starts_with('l') && (has_ctrl || has_opt || has_cmd || has_shift);
    let is_right_only = ak_lower.starts_with('r') && (has_ctrl || has_opt || has_cmd || has_shift);

    if has_ctrl && kid_lower.contains("ctrl") {
        if is_left_only && !kid_lower.contains('l') { return false; }
        if is_right_only && !kid_lower.contains('r') { return false; }
        return true;
    }
    if has_opt && kid_lower.contains("opt") {
        if is_left_only && !kid_lower.contains('l') { return false; }
        if is_right_only && !kid_lower.contains('r') { return false; }
        return true;
    }
    if has_cmd && kid_lower.contains("cmd") {
        if is_left_only && !kid_lower.contains('l') { return false; }
        if is_right_only && !kid_lower.contains('r') { return false; }
        return true;
    }
    if has_shift && kid_lower.contains("shift") {
        if is_left_only && !kid_lower.contains('l') { return false; }
        if is_right_only && !kid_lower.contains('r') { return false; }
        return true;
    }

    // 3. Сочетания (например "ctrl + a")
    // Специальные клавиши и модификаторы не должны разбираться на отдельные буквы
    let special_keys = [
        "hyper", "enter", "return", "space", "tab", "backspace", "esc", "del",
        "pgup", "pgdn", "home", "end", "fn", "up", "down", "left", "right",
        "cmd", "lcmd", "rcmd", "opt", "lopt", "ropt", "ctrl", "lctrl", "rctrl", "shift", "lshift", "rshift"
    ];
    
    if ak_lower.chars().count() > 1 && !special_keys.contains(&ak_lower.as_str()) {
        if key_id.chars().count() == 1 && ak_lower.contains(kid_lower.chars().next().unwrap()) {
            return true;
        }
    }

    false
}

// --- Наша новая клавиатура Keychron K3 ---
const KEYBOARD_LAYOUT: &[&[(&str, &str, usize)]] = &[
    &[
        ("⎋", "esc", 9),
        ("F1", "F1", 6),
        ("F2", "F2", 6),
        ("F3", "F3", 6),
        ("F4", "F4", 6),
        ("F5", "F5", 6),
        ("F6", "F6", 6),
        ("F7", "F7", 6),
        ("F8", "F8", 6),
        ("F9", "F9", 6),
        ("F10", "F10", 6),
        ("F11", "F11", 6),
        ("F12", "F12", 6),
        ("⌦", "del", 6),
        ("🔆", "☀", 9),
    ],
    &[
        ("ˋ", "~", 6),
        ("1", "1", 6),
        ("2", "2", 6),
        ("3", "3", 6),
        ("4", "4", 6),
        ("5", "5", 6),
        ("6", "6", 6),
        ("7", "7", 6),
        ("8", "8", 6),
        ("9", "9", 6),
        ("0", "0", 6),
        ("-", "-", 6),
        ("=", "=", 6),
        ("⌫", "backspace", 12),
        ("⇞", "pgup", 6),
    ],
    &[
        ("⇥", "tab", 9),
        ("q", "Q", 6),
        ("w", "W", 6),
        ("e", "E", 6),
        ("r", "R", 6),
        ("t", "T", 6),
        ("y", "Y", 6),
        ("u", "U", 6),
        ("i", "I", 6),
        ("o", "O", 6),
        ("p", "P", 6),
        ("[", "[", 6),
        ("]", "]", 6),
        ("\\", "\\", 9),
        ("⇟", "pgdn", 6),
    ],
    &[
        ("⇪", "caps", 12),
        ("a", "A", 6),
        ("s", "S", 6),
        ("d", "D", 6),
        ("f", "F", 6),
        ("g", "G", 6),
        ("h", "H", 6),
        ("j", "J", 6),
        ("k", "K", 6),
        ("l", "L", 6),
        (";", ";", 6),
        ("'", "'", 6),
        ("↩", "return", 12),
        ("↖", "home", 6),
    ],
    &[
        ("L⇧", "shift", 15),
        ("z", "Z", 6),
        ("x", "X", 6),
        ("c", "C", 6),
        ("v", "V", 6),
        ("b", "B", 6),
        ("n", "N", 6),
        ("m", "M", 6),
        (",", ",", 6),
        (".", ".", 6),
        ("/", "/", 6),
        ("R⇧", "shift", 9),
        ("↑", "↑", 6),
        ("↘", "end", 6),
    ],
    &[
        ("L⌃", "ctrl", 9),
        ("L⌥", "opt", 9),
        ("L⌘", "cmd", 9),
        ("␣", "space", 33),
        ("R⌘", "cmd", 6),
        ("fn", "fn", 6),
        ("R⌃", "ctrl", 6),
        ("←", "←", 6),
        ("↓", "↓", 6),
        ("→", "→", 6),
    ],
];

pub fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(15),
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let filtered = app.filtered_items();
    let selected_idx = app.state.selected().unwrap_or(0);

    let (active_keys, active_source) = if app.bulk_highlight {
        let all_keys: Vec<String> = filtered
            .iter()
            .flat_map(|i| i.keys.iter().cloned())
            .collect();
        (all_keys, "all") // Используем нейтральный цвет для массовой подсветки
    } else if let Some(item) = filtered.get(selected_idx) {
        (item.keys.clone(), item.source.as_str())
    } else {
        (vec![], "")
    };

    draw_keyboard(f, rects[0], &active_keys, active_source);

    // Рассчитываем примерную ширину колонки описания для переноса текста
    let desc_column_width = (area.width as usize).saturating_sub(60).max(20);

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = Some(i) == app.state.selected();
            let height = if is_selected { 3 } else { 1 };

            let desc = if is_selected {
                wrap_text(&item.desc, desc_column_width)
            } else {
                item.desc.clone()
            };

            let display_source = item.source.split_whitespace()
                .filter(|tag| !tag.contains("_e") && !tag.contains("_d"))
                .collect::<Vec<_>>()
                .join(" ");

            Row::new(vec![
                Cell::from(display_source),
                Cell::from(item.trigger.clone()),
                Cell::from(item.action.clone()),
                Cell::from(Text::from(desc)),
            ]).height(height)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Percentage(30),
            Constraint::Min(40),
        ],
    )
    .header(
        Row::new(vec!["Source", "Trigger", "Action", "Description"])
            .style(Style::default().fg(Color::DarkGray)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Shortcuts (Total: {}) ", filtered.len())),
    )
    .row_highlight_style(selected_style);

    f.render_stateful_widget(table, rects[1], &mut app.state);

    let footer_text = Line::from(vec![
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw("uit  "),
        Span::styled("[?]", Style::default().fg(Color::Cyan)),
        Span::raw("help  "),
        Span::styled("[p]", Style::default().fg(Color::Cyan)),
        Span::raw("arse  "),
        Span::styled("[s]", Style::default().fg(Color::Cyan)),
        Span::raw("ort  "),
        Span::styled("[r]", Style::default().fg(Color::Cyan)),
        Span::raw("eload  "),
        Span::styled("[/]", Style::default().fg(Color::Cyan)),
        Span::raw("search  "),
        Span::styled("[j/k]", Style::default().fg(Color::Cyan)),
        Span::raw(" nav  |  Filters: "),
        Span::styled("[1]", Style::default().fg(Color::Cyan)),
        Span::raw(" All  "),
        Span::styled("[2]", Style::default().fg(Color::Magenta)),
        Span::raw(" skhd  "),
        Span::styled("[3]", Style::default().fg(Color::Blue)),
        Span::raw(" Karabiner  "),
        Span::styled("[4]", Style::default().fg(Color::Green)),
        Span::raw(" Xcode  "),
        Span::styled("[5]", Style::default().fg(Color::Yellow)),
        Span::raw(" System"),
    ]);
    f.render_widget(Paragraph::new(footer_text), rects[2]);

    let search_style = if app.is_searching {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let search_text = format!(" Search: {}", app.search_query);
    f.render_widget(
        Paragraph::new(Span::styled(search_text, search_style)),
        rects[3],
    );

    if app.show_help {
        let area = centered_rect(60, 60, f.area());
        f.render_widget(Clear, area);
        let help_text = vec![
            Line::from(vec![Span::styled(
                " Keyboard Shortcuts ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [q]      ", Style::default().fg(Color::Cyan)),
                Span::raw("Quit Application"),
            ]),
            Line::from(vec![
                Span::styled(" [?]      ", Style::default().fg(Color::Cyan)),
                Span::raw("Toggle Help Menu"),
            ]),
            Line::from(vec![
                Span::styled(" [/]      ", Style::default().fg(Color::Cyan)),
                Span::raw("Enter Search Mode"),
            ]),
            Line::from(vec![
                Span::styled(" [p]      ", Style::default().fg(Color::Cyan)),
                Span::raw("Parse Configs"),
            ]),
            Line::from(vec![
                Span::styled(" [s]      ", Style::default().fg(Color::Cyan)),
                Span::raw("Sort Alphabetically"),
            ]),
            Line::from(vec![
                Span::styled(" [r]      ", Style::default().fg(Color::Cyan)),
                Span::raw("Reload Configuration"),
            ]),
            Line::from(vec![
                Span::styled(" [j/↓]    ", Style::default().fg(Color::Cyan)),
                Span::raw("Next Shortcut"),
            ]),
            Line::from(vec![
                Span::styled(" [k/↑]    ", Style::default().fg(Color::Cyan)),
                Span::raw("Previous Shortcut"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [1]      ", Style::default().fg(Color::Cyan)),
                Span::raw("Filter: All"),
            ]),
            Line::from(vec![
                Span::styled(" [2]      ", Style::default().fg(Color::Magenta)),
                Span::raw("Filter: Skhd"),
            ]),
            Line::from(vec![
                Span::styled(" [3]      ", Style::default().fg(Color::Blue)),
                Span::raw("Filter: Karabiner"),
            ]),
            Line::from(vec![
                Span::styled(" [4]      ", Style::default().fg(Color::Green)),
                Span::raw("Filter: Xcode"),
            ]),
            Line::from(vec![
                Span::styled(" [5]      ", Style::default().fg(Color::Yellow)),
                Span::raw("Filter: System"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                " Search Mode: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::raw("  Type to filter, "),
                Span::styled("[Enter/Esc]", Style::default().fg(Color::Cyan)),
                Span::raw(" to exit"),
            ]),
        ];
        let help_block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        f.render_widget(Paragraph::new(help_text).block(help_block), area);
    }
}

fn draw_keyboard(f: &mut Frame, area: Rect, active_keys: &[String], source: &str) {
    let active_color = match source.to_lowercase() {
        s if s.contains("sk") => Color::Magenta,
        s if s.contains("ke") => Color::Cyan,
        s if s.contains("xcode") => Color::Green,
        s if s.contains("sy") => Color::Yellow,
        _ => Color::White,
    };

    let mut text_lines = vec![];
    let row_separator = "-".repeat(97);

    let is_hyper = active_keys.iter().any(|k| k == "Hyper");

    for (row_idx, row) in KEYBOARD_LAYOUT.iter().enumerate() {
        let mut spans = vec![];

        let row_active: Vec<bool> = row
            .iter()
            .map(|&(key_id, display, _)| {
                active_keys
                    .iter()
                    .any(|k| is_key_matched(k, key_id, display))
                    || (is_hyper && ["L⌘", "L⌥", "L⌃", "L⇧"].contains(&key_id))
            })
            .collect();

        for (i, &(key_id, display, width)) in row.iter().enumerate() {
            let is_esc = key_id == "⎋";
            let is_active = row_active[i];
            let prev_is_active = if i > 0 { row_active[i - 1] } else { false };

            // Левый или разделительный слэш
            let slash_active = is_active || prev_is_active;
            let slash_style = if slash_active {
                Style::default()
                    .fg(active_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled("/", slash_style));

            let color = if is_esc {
                Color::Rgb(255, 165, 0) // Orange
            } else if is_active {
                active_color
            } else {
                Color::DarkGray
            };

            let style = if is_active || is_esc {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            let inner_width = width - 1;
            let text_chars = display.chars().count();

            let left_pad = inner_width.saturating_sub(text_chars) / 2;
            let right_pad = inner_width.saturating_sub(text_chars) - left_pad;

            let key_text = format!(
                "{spaces_left}{text}{spaces_right}",
                spaces_left = " ".repeat(left_pad),
                text = display,
                spaces_right = " ".repeat(right_pad)
            );

            spans.push(Span::styled(key_text, style));

            // Завершающий слэш в ряду
            if i == row.len() - 1 {
                let last_slash_style = if is_active {
                    Style::default()
                        .fg(active_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled("/", last_slash_style));
            }
        }

        text_lines.push(Line::from(spans));

        if row_idx < KEYBOARD_LAYOUT.len() - 1 {
            text_lines.push(Line::from(Span::styled(
                &row_separator,
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Keychron K3 Layout ");

    let paragraph = Paragraph::new(text_lines).block(block);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
