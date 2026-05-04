use std::fs;

fn main() {
    let content = fs::read_to_string("src/shortcuts.json").unwrap();
    let karabiner_count = content.lines().filter(|l| l.contains("\"source\":") && l.contains("karabiner")).count();
    println!("Karabiner sources: {}", karabiner_count);
}
