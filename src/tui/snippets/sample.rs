//! Syntax preview: comments, keywords, types, strings, escapes, labels.

use std::collections::HashMap;

const MAX_ITEMS: usize = 100;
static VERSION: &str = "1.0.0";

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub count: u32,
    pub enabled: bool,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            count: 0,
            enabled: true,
        }
    }
}

fn process(items: &[i32]) -> HashMap<i32, bool> {
    let mut result = HashMap::new();
    'outer: for &item in items {
        if item < 0 {
            continue 'outer;
        }
        let is_even = item % 2 == 0;
        result.insert(item, is_even);
    }
    result
}

fn main() {
    let msg = "Hello\tWorld\n";
    let pattern = regex::Regex::new(r"\d+").unwrap();
    let config = Config::new("example");
    println!("Config: {:?}, msg: {}, pattern: {}", config, msg, pattern);
    let processed = process(&[1, 2, -3, 4, 5]);
    println!("Result: {:?}", processed);
}
