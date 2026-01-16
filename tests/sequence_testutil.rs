use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct TestCase {
    pub mermaid: String,
    pub expected: String,
}

pub fn read_sequence_test_case<P: AsRef<Path>>(path: P) -> Result<TestCase, String> {
    let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let parts: Vec<&str> = contents.split("\n---\n").collect();
    if parts.len() != 2 {
        return Err("test case file must have exactly one '---' separator".to_string());
    }
    Ok(TestCase {
        mermaid: parts[0].trim().to_string(),
        expected: parts[1].trim().to_string(),
    })
}

pub fn normalize_whitespace(input: &str) -> String {
    let mut normalized = Vec::new();
    for line in input.lines() {
        let trimmed = line.trim_end_matches(' ');
        if !trimmed.is_empty() || !normalized.is_empty() {
            normalized.push(trimmed.to_string());
        }
    }
    while normalized.last().map(|s| s.is_empty()).unwrap_or(false) {
        normalized.pop();
    }
    normalized.join("\n")
}

pub fn visualize_whitespace(input: &str) -> String {
    input.replace(' ', "·")
}
