mod sequence_testutil;

use console_mermaid::diagram::Config;
use console_mermaid::sequence::{parse, render};
use std::path::Path;

fn verify_sequence<P: AsRef<Path>>(path: P, use_ascii: bool) {
    let tc = sequence_testutil::read_sequence_test_case(path).expect("read sequence test");
    let diagram = parse(&tc.mermaid).expect("parse sequence");
    let config = Config::new_test_config(use_ascii, "cli");
    let output = render(&diagram, &config).expect("render sequence");

    let expected = sequence_testutil::normalize_whitespace(&tc.expected);
    let actual = sequence_testutil::normalize_whitespace(&output);
    if expected != actual {
        let expected_dbg = sequence_testutil::visualize_whitespace(&expected);
        let actual_dbg = sequence_testutil::visualize_whitespace(&actual);
        panic!(
            "Sequence diagram mismatch\nExpected:\n{}\nActual:\n{}",
            expected_dbg, actual_dbg
        );
    }
}

#[test]
fn test_sequence_unicode_golden() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/sequence");
    let files = [
        "adjacent_participants_communication.txt",
        "autonumber.txt",
        "bidirectional_messages.txt",
        "dotted_arrows_only.txt",
        "four_participants.txt",
        "long_participant_names.txt",
        "messages_without_labels.txt",
        "multiword_labels.txt",
        "self_message.txt",
        "simple_two_participants.txt",
        "single_message.txt",
        "three_participants.txt",
    ];
    for file in files {
        verify_sequence(base.join(file), false);
    }
}

#[test]
fn test_sequence_ascii_golden() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/sequence-ascii");
    let files = [
        "autonumber.txt",
        "dotted_arrows_only.txt",
        "self_message.txt",
        "simple_two_participants.txt",
        "three_participants.txt",
    ];
    for file in files {
        verify_sequence(base.join(file), true);
    }
}

#[test]
fn test_sequence_ascii_smoke() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testdata/sequence");
    let files = [
        "adjacent_participants_communication.txt",
        "autonumber.txt",
        "bidirectional_messages.txt",
        "dotted_arrows_only.txt",
        "four_participants.txt",
        "long_participant_names.txt",
        "messages_without_labels.txt",
        "multiword_labels.txt",
        "self_message.txt",
        "simple_two_participants.txt",
        "single_message.txt",
        "three_participants.txt",
    ];

    for file in files {
        let tc = sequence_testutil::read_sequence_test_case(base.join(file)).expect("read test");
        let diagram = parse(&tc.mermaid).expect("parse");
        let config = Config::new_test_config(true, "cli");
        let output = render(&diagram, &config).expect("render");
        assert!(!output.trim().is_empty(), "ASCII output is empty");
        for participant in &diagram.participants {
            assert!(output.contains(&participant.label));
        }
        assert!(
            !output.contains('┌')
                && !output.contains('┐')
                && !output.contains('└')
                && !output.contains('┘')
        );
    }
}
