use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use clap::{CommandFactory, Parser};

#[derive(Parser, Debug)]
#[command(name = "console-mermaid")]
#[command(about = "Render Mermaid diagrams to ASCII/Unicode in the terminal")]
struct Cli {
    /// Input file path, or "-" to read from stdin
    input: Option<PathBuf>,

    /// Use ASCII-only characters
    #[arg(long)]
    ascii: bool,

    /// Show layout coordinates
    #[arg(long)]
    coords: bool,

    /// Enable verbose logging in rendering
    #[arg(long)]
    verbose: bool,

    /// Padding inside node boxes
    #[arg(long, default_value_t = console_mermaid::diagram::Config::default_config().box_border_padding)]
    box_padding: i32,

    /// Horizontal padding between nodes
    #[arg(long, default_value_t = console_mermaid::diagram::Config::default_config().padding_between_x)]
    padding_x: i32,

    /// Vertical padding between nodes
    #[arg(long, default_value_t = console_mermaid::diagram::Config::default_config().padding_between_y)]
    padding_y: i32,

    /// Graph direction: LR or TD
    #[arg(long, default_value = "LR", value_parser = ["LR", "TD"])]
    graph_direction: String,
}

fn main() {
    let cli = Cli::parse();

    let mut input = String::new();
    match cli.input {
        Some(path) if path.as_os_str() == "-" => {
            if io::stdin().read_to_string(&mut input).is_err() {
                eprintln!("failed to read stdin");
                std::process::exit(1);
            }
        }
        Some(path) => {
            match std::fs::read_to_string(&path) {
                Ok(contents) => input = contents,
                Err(err) => {
                    eprintln!("failed to read {}: {}", path.display(), err);
                    std::process::exit(1);
                }
            }
        }
        None => {
            if io::stdin().is_terminal() {
                eprintln!("no input provided; pass a file path or '-' for stdin");
                let mut cmd = Cli::command();
                let _ = cmd.print_help();
                eprintln!();
                std::process::exit(2);
            }
            if io::stdin().read_to_string(&mut input).is_err() {
                eprintln!("failed to read stdin");
                std::process::exit(1);
            }
        }
    }

    if input.trim().is_empty() {
        eprintln!("no input provided");
        std::process::exit(1);
    }

    let config = match console_mermaid::diagram::Config::new_cli_config(
        cli.ascii,
        cli.coords,
        cli.verbose,
        cli.box_padding,
        cli.padding_x,
        cli.padding_y,
        cli.graph_direction,
    ) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };
    match console_mermaid::render_diagram(&input, &config) {
        Ok(output) => println!("{}", output),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}
