use std::fs;

#[derive(serde::Deserialize, Debug)]
pub struct Shortcut {
    pub source: String,
    #[serde(default)]
    pub rules: String,
    pub keys: Vec<String>,
    pub action: String,
    pub desc: String,
}

fn main() {
    let content = fs::read_to_string("src/shortcuts.json").unwrap();
    let items: Vec<Shortcut> = serde_json::from_str(&content).unwrap();

    let app_q = "xc".to_string();
    let filter_str = "karabiner";
    let is_filtering_app = true;

    let filtered: Vec<&Shortcut> = items.iter().filter(|i| {
        if !i.source.contains(filter_str) {
            return false;
        }

        if !app_q.is_empty() {
            let enable_tag = format!("{}_e", app_q);
            let disable_tag = format!("{}_d", app_q);
            if !i.rules.contains(&enable_tag) && !i.rules.contains(&disable_tag) {
                return false;
            }
        }
        true
    }).collect();

    println!("Filtered count: {}", filtered.len());
    for item in filtered.iter().take(3) {
        println!("{:?}", item);
    }
}
