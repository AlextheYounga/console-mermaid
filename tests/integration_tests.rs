use console_mermaid::diagram::Config;
use console_mermaid::render_diagram;

#[test]
fn test_sequence_diagram_integration() {
    let input = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi";
    let config = Config::new_test_config(false, "cli");
    let output = render_diagram(input, &config).expect("render");
    assert!(output.contains("Alice"));
    assert!(output.contains("Bob"));
    assert!(output.contains("Hello"));
    assert!(output.contains("Hi"));
}

#[test]
fn test_sequence_ascii_integration() {
    let input = "sequenceDiagram\n    Alice->>Bob: Hello";
    let config = Config::new_test_config(true, "cli");
    let output = render_diagram(input, &config).expect("render");
    assert!(output.contains('+'));
    assert!(output.contains('|'));
    assert!(!output.contains('│'));
}

#[test]
fn test_invalid_input_errors() {
    let config = Config::new_test_config(false, "cli");
    assert!(render_diagram("", &config).is_err());
    assert!(render_diagram("not a diagram", &config).is_err());
}
