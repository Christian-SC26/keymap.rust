fn main() {
    let keys = vec!["lcmd".to_string(), "o".to_string()];
    let active_modifiers = vec!["cmd".to_string()];
    
    let mut has_all = true;
    for m in &active_modifiers {
        if !keys.iter().any(|k| k.to_lowercase().contains(m)) {
            has_all = false;
            break;
        }
    }
    println!("has_all: {}", has_all);
}
