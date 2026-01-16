mod draw;
mod layout;
mod parse;
mod types;

use crate::diagram::{Config, Diagram};
use types::GraphProperties;

#[derive(Debug, Clone, Default)]
pub struct GraphDiagram {
    properties: Option<GraphProperties>,
}

impl Diagram for GraphDiagram {
    fn parse(&mut self, input: &str, config: &Config) -> Result<(), String> {
        let properties = parse::mermaid_to_graph_properties(input, "cli", config)?;
        self.properties = Some(properties);
        Ok(())
    }

    fn render(&self, config: &Config) -> Result<String, String> {
        let mut properties = self
            .properties
            .clone()
            .ok_or_else(|| "graph diagram not parsed: call parse() before render()".to_string())?;
        let style_type = if config.style_type.is_empty() {
            "cli".to_string()
        } else {
            config.style_type.clone()
        };
        properties.style_type = style_type;
        properties.use_ascii = config.use_ascii;
        draw::draw_map(&properties, config.show_coords)
    }

    fn diagram_type(&self) -> &'static str {
        "graph"
    }
}
