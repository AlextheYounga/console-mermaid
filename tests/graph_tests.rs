mod testutil;

use console_mermaid::diagram::Config;
use console_mermaid::render_diagram;
use std::fs;
use std::path::Path;

fn verify_map<P: AsRef<Path>>(path: P, use_ascii: bool) {
    let tc = testutil::read_test_case(path).expect("read test case");
    let mut config = Config::default_config();
    config.use_ascii = use_ascii;
    config.padding_between_x = tc.padding_x;
    config.padding_between_y = tc.padding_y;
    config.style_type = "cli".to_string();

    let output = render_diagram(&tc.mermaid, &config).expect("render diagram");
    if tc.expected != output {
        let expected = testutil::visualize_whitespace(&tc.expected);
        let actual = testutil::visualize_whitespace(&output);
        panic!("Map didn't match\nExpected:\n{}\nActual:\n{}", expected, actual);
    }
}

#[test]
fn test_ascii() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/ascii");
    for entry in fs::read_dir(dir).expect("read ascii dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            verify_map(path, true);
        }
    }
}

#[test]
fn test_extended_chars() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/extended-chars");
    for entry in fs::read_dir(dir).expect("read extended dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            verify_map(path, false);
        }
    }
}

#[test]
fn test_graph_use_ascii_config() {
    let input = "graph LR\nA --> B";

    let mut ascii_config = Config::default_config();
    ascii_config.use_ascii = true;
    let ascii_output = render_diagram(input, &ascii_config).expect("render ascii");

    let mut unicode_config = Config::default_config();
    unicode_config.use_ascii = false;
    let unicode_output = render_diagram(input, &unicode_config).expect("render unicode");

    assert_ne!(ascii_output, unicode_output, "ASCII and Unicode outputs should differ");
    assert!(!ascii_output.contains('┌') && !ascii_output.contains('─') && !ascii_output.contains('│'));
    assert!(unicode_output.contains('┌') || unicode_output.contains('─') || unicode_output.contains('│'));
}
