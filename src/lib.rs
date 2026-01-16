pub mod diagram;
pub mod graph;
pub mod sequence;

pub fn render_diagram(input: &str, config: &diagram::Config) -> Result<String, String> {
    let mut diag = diagram::diagram_factory(input)?;
    diag.parse(input, config)?;
    diag.render(config)
}
