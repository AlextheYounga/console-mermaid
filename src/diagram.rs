use crate::graph::GraphDiagram;
use crate::sequence::SequenceDiagram;

pub trait Diagram {
    fn parse(&mut self, input: &str, config: &Config) -> Result<(), String>;
    fn render(&self, config: &Config) -> Result<String, String>;
    fn diagram_type(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct Config {
    pub use_ascii: bool,
    pub show_coords: bool,
    pub verbose: bool,
    pub box_border_padding: i32,
    pub padding_between_x: i32,
    pub padding_between_y: i32,
    pub graph_direction: String,
    pub style_type: String,
    pub sequence_participant_spacing: i32,
    pub sequence_message_spacing: i32,
    pub sequence_self_message_width: i32,
}

#[derive(Debug)]
pub struct ConfigError {
    pub field: &'static str,
    pub value: String,
    pub message: &'static str,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid config: {} = {} ({})",
            self.field, self.value, self.message
        )
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn default_config() -> Self {
        Self {
            use_ascii: false,
            show_coords: false,
            verbose: false,
            box_border_padding: 1,
            padding_between_x: 5,
            padding_between_y: 5,
            graph_direction: "LR".to_string(),
            style_type: "cli".to_string(),
            sequence_participant_spacing: 5,
            sequence_message_spacing: 1,
            sequence_self_message_width: 4,
        }
    }

    pub fn new_cli_config(
        use_ascii: bool,
        show_coords: bool,
        verbose: bool,
        box_border_padding: i32,
        padding_x: i32,
        padding_y: i32,
        graph_direction: String,
    ) -> Result<Self, String> {
        let defaults = Self::default_config();
        let config = Self {
            use_ascii,
            show_coords,
            verbose,
            box_border_padding,
            padding_between_x: padding_x,
            padding_between_y: padding_y,
            graph_direction,
            style_type: "cli".to_string(),
            sequence_participant_spacing: defaults.sequence_participant_spacing,
            sequence_message_spacing: defaults.sequence_message_spacing,
            sequence_self_message_width: defaults.sequence_self_message_width,
        };

        config.validate()?;
        Ok(config)
    }

    pub fn new_test_config(use_ascii: bool, style_type: &str) -> Self {
        let mut config = Self::default_config();
        config.use_ascii = use_ascii;
        config.style_type = style_type.to_string();
        config
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.box_border_padding < 0 {
            return Err(ConfigError {
                field: "box_border_padding",
                value: self.box_border_padding.to_string(),
                message: "must be non-negative",
            }
            .to_string());
        }
        if self.padding_between_x < 0 {
            return Err(ConfigError {
                field: "padding_between_x",
                value: self.padding_between_x.to_string(),
                message: "must be non-negative",
            }
            .to_string());
        }
        if self.padding_between_y < 0 {
            return Err(ConfigError {
                field: "padding_between_y",
                value: self.padding_between_y.to_string(),
                message: "must be non-negative",
            }
            .to_string());
        }
        if self.graph_direction != "LR" && self.graph_direction != "TD" {
            return Err(ConfigError {
                field: "graph_direction",
                value: self.graph_direction.clone(),
                message: "must be \"LR\" or \"TD\"",
            }
            .to_string());
        }
        if self.style_type != "cli" && self.style_type != "html" {
            return Err(ConfigError {
                field: "style_type",
                value: self.style_type.clone(),
                message: "must be \"cli\" or \"html\"",
            }
            .to_string());
        }
        if self.sequence_participant_spacing < 0 {
            return Err(ConfigError {
                field: "sequence_participant_spacing",
                value: self.sequence_participant_spacing.to_string(),
                message: "must be non-negative",
            }
            .to_string());
        }
        if self.sequence_message_spacing < 0 {
            return Err(ConfigError {
                field: "sequence_message_spacing",
                value: self.sequence_message_spacing.to_string(),
                message: "must be non-negative",
            }
            .to_string());
        }
        if self.sequence_self_message_width < 2 {
            return Err(ConfigError {
                field: "sequence_self_message_width",
                value: self.sequence_self_message_width.to_string(),
                message: "must be at least 2",
            }
            .to_string());
        }

        Ok(())
    }
}

pub fn diagram_factory(input: &str) -> Result<Box<dyn Diagram>, String> {
    let input = input.trim();
    if crate::sequence::is_sequence_diagram(input) {
        return Ok(Box::new(SequenceDiagram::default()));
    }

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        if trimmed.starts_with("graph ") || trimmed.starts_with("flowchart ") {
            return Ok(Box::new(GraphDiagram::default()));
        }
        if !trimmed.starts_with("%%") {
            return Ok(Box::new(GraphDiagram::default()));
        }
    }

    Ok(Box::new(GraphDiagram::default()))
}

pub fn split_lines(input: &str) -> Vec<String> {
    let re = regex::Regex::new(r"\n|\\n").unwrap();
    re.split(input).map(|s| s.to_string()).collect()
}

pub fn remove_comments(lines: &[String]) -> Vec<String> {
    let mut cleaned = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("%%") {
            continue;
        }
        let mut current = line.clone();
        if let Some(idx) = current.find("%%") {
            current = current[..idx].trim().to_string();
        }
        if !current.trim().is_empty() {
            cleaned.push(current);
        }
    }
    cleaned
}
