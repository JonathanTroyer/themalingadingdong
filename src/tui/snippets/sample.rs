//! Example Rust code for syntax highlighting preview.

use std::collections::HashMap;

const MAX_RETRIES: u32 = 3;
const API_URL: &str = "https://api.example.com";

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub enabled: bool,
    pub retries: u32,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            retries: MAX_RETRIES,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Name cannot be empty".into());
        }
        if self.retries > 10 {
            return Err(format!("Retries {} exceeds maximum", self.retries));
        }
        Ok(())
    }
}

fn process_items(items: &[i32]) -> HashMap<i32, bool> {
    let mut result = HashMap::new();
    for &item in items {
        let is_even = item % 2 == 0;
        result.insert(item, is_even);
    }
    result
}

fn main() {
    let config = Config::new("example");
    println!("Config: {:?}", config);

    let items = vec![1, 2, 3, 4, 5];
    let processed = process_items(&items);
    println!("Processed: {:?}", processed);
}
