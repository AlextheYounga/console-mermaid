use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct TestCase {
    pub mermaid: String,
    pub expected: String,
    pub padding_x: i32,
    pub padding_y: i32,
}

pub fn read_test_case<P: AsRef<Path>>(path: P) -> Result<TestCase, String> {
    let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut mermaid = String::new();
    let mut expected = String::new();
    let mut in_mermaid = true;
    let mut mermaid_started = false;
    let mut padding_x = 5;
    let mut padding_y = 5;

    let padding_re = regex::Regex::new(r"(?i)^(padding[xy])\s*=\s*(\d+)\s*$").unwrap();

    for line in contents.lines() {
        if line == "---" {
            in_mermaid = false;
            continue;
        }
        if in_mermaid {
            let trimmed = line.trim();
            if !mermaid_started {
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(caps) = padding_re.captures(trimmed) {
                    let value: i32 = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<i32>()
                        .map_err(|e| e.to_string())?;
                    if caps
                        .get(1)
                        .unwrap()
                        .as_str()
                        .eq_ignore_ascii_case("paddingX")
                    {
                        padding_x = value;
                    } else {
                        padding_y = value;
                    }
                    continue;
                }
            }
            mermaid_started = true;
            mermaid.push_str(line);
            mermaid.push('\n');
        } else {
            expected.push_str(line);
            expected.push('\n');
        }
    }

    Ok(TestCase {
        mermaid,
        expected: expected.trim_end_matches('\n').to_string(),
        padding_x,
        padding_y,
    })
}

pub fn visualize_whitespace(input: &str) -> String {
    input.replace(' ', "·")
}
