use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("failed to read stdin");
        std::process::exit(1);
    }

    if input.trim().is_empty() {
        eprintln!("no input provided on stdin");
        std::process::exit(1);
    }

    let config = console_mermaid::diagram::Config::default_config();
    match console_mermaid::render_diagram(&input, &config) {
        Ok(output) => println!("{}", output),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}
