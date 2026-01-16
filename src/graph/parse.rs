use crate::diagram::Config;
use crate::graph::types::{GraphProperties, StyleClass, TextEdge, TextNode, TextSubgraph};
use indexmap::IndexMap;
use log::debug;
use regex::Regex;
use std::collections::HashSet;

pub(crate) fn mermaid_to_graph_properties(
    mermaid: &str,
    style_type: &str,
    config: &Config,
) -> Result<GraphProperties, String> {
    let newline_re = Regex::new(r"\n|\\n").unwrap();
    let raw_lines: Vec<String> = newline_re.split(mermaid).map(|s| s.to_string()).collect();

    let mut lines: Vec<String> = Vec::new();
    for mut line in raw_lines {
        if line == "---" {
            break;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("%%") {
            continue;
        }
        if let Some(idx) = line.find("%%") {
            line = line[..idx].trim().to_string();
        }
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }

    let mut properties = GraphProperties {
        data: IndexMap::new(),
        style_classes: std::collections::HashMap::new(),
        graph_direction: String::new(),
        style_type: style_type.to_string(),
        padding_x: config.padding_between_x,
        padding_y: config.padding_between_y,
        box_border_padding: config.box_border_padding,
        subgraphs: Vec::new(),
        use_ascii: config.use_ascii,
    };

    let padding_re = Regex::new(r"(?i)^padding([xy])\s*=\s*(\d+)$").unwrap();
    while !lines.is_empty() {
        let trimmed = lines[0].trim();
        if trimmed.is_empty() {
            lines.remove(0);
            continue;
        }
        if let Some(caps) = padding_re.captures(trimmed) {
            let axis = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let value: i32 = caps
                .get(2)
                .unwrap()
                .as_str()
                .parse::<i32>()
                .map_err(|e| e.to_string())?;
            if axis.eq_ignore_ascii_case("x") {
                properties.padding_x = value;
            } else {
                properties.padding_y = value;
            }
            lines.remove(0);
            continue;
        }
        break;
    }

    if lines.is_empty() {
        return Err("missing graph definition".to_string());
    }

    match lines[0].as_str() {
        "graph LR" | "flowchart LR" => properties.graph_direction = "LR".to_string(),
        "graph TD" | "flowchart TD" | "graph TB" | "flowchart TB" => {
            properties.graph_direction = "TD".to_string()
        }
        other => {
            return Err(format!(
                "unsupported graph type '{}'. Supported types: graph TD, graph TB, graph LR, flowchart TD, flowchart TB, flowchart LR",
                other
            ));
        }
    }
    lines.remove(0);

    let subgraph_re = Regex::new(r"^\s*subgraph\s+(.+)$").unwrap();
    let end_re = Regex::new(r"^\s*end\s*$").unwrap();
    let mut subgraph_stack: Vec<usize> = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        if let Some(caps) = subgraph_re.captures(trimmed) {
            let name = caps.get(1).unwrap().as_str().trim().to_string();
            let parent = subgraph_stack.last().copied();
            let idx = properties.subgraphs.len();
            properties.subgraphs.push(TextSubgraph {
                name,
                nodes: Vec::new(),
                parent,
                children: Vec::new(),
            });
            if let Some(parent_idx) = parent {
                properties.subgraphs[parent_idx].children.push(idx);
            }
            subgraph_stack.push(idx);
            continue;
        }

        if end_re.is_match(trimmed) {
            subgraph_stack.pop();
            continue;
        }

        let existing_nodes: HashSet<String> = properties.data.keys().cloned().collect();

        if let Ok(nodes) = properties.parse_string(&line) {
            for node in nodes {
                add_node(&node, &mut properties.data);
            }
        } else {
            let node = parse_node(&line);
            add_node(&node, &mut properties.data);
        }

        if !subgraph_stack.is_empty() {
            for key in properties.data.keys() {
                if !existing_nodes.contains(key) {
                    for idx in &subgraph_stack {
                        let subgraph = &mut properties.subgraphs[*idx];
                        if !subgraph.nodes.contains(key) {
                            subgraph.nodes.push(key.clone());
                        }
                    }
                }
            }
        }
    }

    Ok(properties)
}

impl GraphProperties {
    pub(crate) fn parse_string(&mut self, line: &str) -> Result<Vec<TextNode>, String> {
        debug!("Parsing line: {}", line);
        let line = line.trim();

        if line.is_empty() {
            return Ok(Vec::new());
        }

        let arrow_re = Regex::new(r"^(.+)\s+-->\s+(.+)$").unwrap();
        let label_re = Regex::new(r"^(.+)\s+-->\|(.+)\|\s+(.+)$").unwrap();
        let class_re = Regex::new(r"^classDef\s+(.+)\s+(.+)$").unwrap();
        let amp_re = Regex::new(r"^(.+) & (.+)$").unwrap();

        if let Some(caps) = arrow_re.captures(line) {
            let lhs = caps.get(1).unwrap().as_str();
            let rhs = caps.get(2).unwrap().as_str();
            let left_nodes = self
                .parse_string(lhs)
                .unwrap_or_else(|_| vec![parse_node(lhs)]);
            let right_nodes = self
                .parse_string(rhs)
                .unwrap_or_else(|_| vec![parse_node(rhs)]);
            return Ok(set_arrow(&left_nodes, &right_nodes, &mut self.data));
        }

        if let Some(caps) = label_re.captures(line) {
            let lhs = caps.get(1).unwrap().as_str();
            let label = caps.get(2).unwrap().as_str();
            let rhs = caps.get(3).unwrap().as_str();
            let left_nodes = self
                .parse_string(lhs)
                .unwrap_or_else(|_| vec![parse_node(lhs)]);
            let right_nodes = self
                .parse_string(rhs)
                .unwrap_or_else(|_| vec![parse_node(rhs)]);
            return Ok(set_arrow_with_label(
                &left_nodes,
                &right_nodes,
                label,
                &mut self.data,
            ));
        }

        if let Some(caps) = class_re.captures(line) {
            let class_name = caps.get(1).unwrap().as_str();
            let styles = caps.get(2).unwrap().as_str();
            let class = parse_style_class(class_name, styles);
            self.style_classes.insert(class.name.clone(), class);
            return Ok(Vec::new());
        }

        if let Some(caps) = amp_re.captures(line) {
            let lhs = caps.get(1).unwrap().as_str();
            let rhs = caps.get(2).unwrap().as_str();
            let left_nodes = self
                .parse_string(lhs)
                .unwrap_or_else(|_| vec![parse_node(lhs)]);
            let right_nodes = self
                .parse_string(rhs)
                .unwrap_or_else(|_| vec![parse_node(rhs)]);
            let mut merged = left_nodes;
            merged.extend(right_nodes);
            return Ok(merged);
        }

        Err(format!("could not parse line: {}", line))
    }
}

fn parse_node(line: &str) -> TextNode {
    let trimmed = line.trim();
    let node_re = Regex::new(r"^(.+):::(.+)$").unwrap();
    if let Some(caps) = node_re.captures(trimmed) {
        TextNode {
            name: caps.get(1).unwrap().as_str().trim().to_string(),
            style_class: caps.get(2).unwrap().as_str().trim().to_string(),
        }
    } else {
        TextNode {
            name: trimmed.to_string(),
            style_class: String::new(),
        }
    }
}

fn parse_style_class(name: &str, styles: &str) -> StyleClass {
    let mut style_map = std::collections::HashMap::new();
    for style in styles.split(',') {
        let mut parts = style.splitn(2, ':');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        style_map.insert(key.to_string(), value.to_string());
    }
    StyleClass {
        name: name.to_string(),
        styles: style_map,
    }
}

fn set_arrow_with_label(
    lhs: &[TextNode],
    rhs: &[TextNode],
    label: &str,
    data: &mut IndexMap<String, Vec<TextEdge>>,
) -> Vec<TextNode> {
    debug!(
        "Setting arrow from {:?} to {:?} with label {}",
        lhs, rhs, label
    );
    for l in lhs {
        for r in rhs {
            set_data(
                l,
                TextEdge {
                    parent: l.clone(),
                    child: r.clone(),
                    label: label.to_string(),
                },
                data,
            );
        }
    }
    rhs.to_vec()
}

fn set_arrow(
    lhs: &[TextNode],
    rhs: &[TextNode],
    data: &mut IndexMap<String, Vec<TextEdge>>,
) -> Vec<TextNode> {
    set_arrow_with_label(lhs, rhs, "", data)
}

fn add_node(node: &TextNode, data: &mut IndexMap<String, Vec<TextEdge>>) {
    if !data.contains_key(&node.name) {
        data.insert(node.name.clone(), Vec::new());
    }
}

fn set_data(parent: &TextNode, edge: TextEdge, data: &mut IndexMap<String, Vec<TextEdge>>) {
    if let Some(children) = data.get_mut(&parent.name) {
        children.push(edge.clone());
    } else {
        data.insert(parent.name.clone(), vec![edge.clone()]);
    }
    if !data.contains_key(&edge.child.name) {
        data.insert(edge.child.name.clone(), Vec::new());
    }
}
