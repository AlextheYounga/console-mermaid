use crate::diagram::{Config, Diagram};
use indexmap::IndexMap;
use log::debug;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
struct TextNode {
    name: String,
    style_class: String,
}

#[derive(Debug, Clone)]
struct TextEdge {
    parent: TextNode,
    child: TextNode,
    label: String,
}

#[derive(Debug, Clone)]
struct TextSubgraph {
    name: String,
    nodes: Vec<String>,
    parent: Option<usize>,
    children: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct GraphProperties {
    data: IndexMap<String, Vec<TextEdge>>,
    style_classes: HashMap<String, StyleClass>,
    graph_direction: String,
    style_type: String,
    padding_x: i32,
    padding_y: i32,
    box_border_padding: i32,
    subgraphs: Vec<TextSubgraph>,
    use_ascii: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GraphDiagram {
    properties: Option<GraphProperties>,
}

impl Diagram for GraphDiagram {
    fn parse(&mut self, input: &str, config: &Config) -> Result<(), String> {
        let properties = mermaid_to_graph_properties(input, "cli", config)?;
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
        draw_map(&properties, config.show_coords)
    }

    fn diagram_type(&self) -> &'static str {
        "graph"
    }
}

#[derive(Debug, Clone, Default)]
struct StyleClass {
    name: String,
    styles: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy)]
struct GenericCoord {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, Eq)]
struct GridCoord {
    x: i32,
    y: i32,
}

impl PartialEq for GridCoord {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Hash for GridCoord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

#[derive(Debug, Clone, Copy, Eq)]
struct DrawingCoord {
    x: i32,
    y: i32,
}

impl PartialEq for DrawingCoord {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl GridCoord {
    fn equals(&self, other: GridCoord) -> bool {
        self.x == other.x && self.y == other.y
    }

    fn direction(&self, dir: Direction) -> GridCoord {
        GridCoord {
            x: self.x + dir.dx,
            y: self.y + dir.dy,
        }
    }
}

impl DrawingCoord {
    fn equals(&self, other: DrawingCoord) -> bool {
        self.x == other.x && self.y == other.y
    }

    fn direction(&self, dir: Direction) -> DrawingCoord {
        DrawingCoord {
            x: self.x + dir.dx,
            y: self.y + dir.dy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Direction {
    dx: i32,
    dy: i32,
}

const UP: Direction = Direction { dx: 1, dy: 0 };
const DOWN: Direction = Direction { dx: 1, dy: 2 };
const LEFT: Direction = Direction { dx: 0, dy: 1 };
const RIGHT: Direction = Direction { dx: 2, dy: 1 };
const UPPER_RIGHT: Direction = Direction { dx: 2, dy: 0 };
const UPPER_LEFT: Direction = Direction { dx: 0, dy: 0 };
const LOWER_RIGHT: Direction = Direction { dx: 2, dy: 2 };
const LOWER_LEFT: Direction = Direction { dx: 0, dy: 2 };
const MIDDLE: Direction = Direction { dx: 1, dy: 1 };

impl Direction {
    fn opposite(self) -> Direction {
        match self {
            UP => DOWN,
            DOWN => UP,
            LEFT => RIGHT,
            RIGHT => LEFT,
            UPPER_RIGHT => LOWER_LEFT,
            UPPER_LEFT => LOWER_RIGHT,
            LOWER_RIGHT => UPPER_LEFT,
            LOWER_LEFT => UPPER_RIGHT,
            MIDDLE => MIDDLE,
            _ => MIDDLE,
        }
    }
}

type Drawing = Vec<Vec<String>>;

#[derive(Debug, Clone)]
struct Node {
    name: String,
    drawing: Option<Drawing>,
    drawing_coord: Option<DrawingCoord>,
    grid_coord: Option<GridCoord>,
    drawn: bool,
    index: usize,
    style_class_name: String,
    style_class: StyleClass,
}

#[derive(Debug, Clone)]
struct Edge {
    from: usize,
    to: usize,
    text: String,
    path: Vec<GridCoord>,
    label_line: Vec<GridCoord>,
    start_dir: Direction,
    end_dir: Direction,
}

#[derive(Debug, Clone)]
struct Subgraph {
    name: String,
    nodes: Vec<usize>,
    parent: Option<usize>,
    children: Vec<usize>,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

#[derive(Debug, Clone)]
struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    drawing: Drawing,
    grid: HashMap<GridCoord, usize>,
    column_width: HashMap<i32, i32>,
    row_height: HashMap<i32, i32>,
    style_classes: HashMap<String, StyleClass>,
    style_type: String,
    padding_x: i32,
    padding_y: i32,
    box_border_padding: i32,
    subgraphs: Vec<Subgraph>,
    offset_x: i32,
    offset_y: i32,
    use_ascii: bool,
    graph_direction: String,
    node_index_by_name: HashMap<String, usize>,
}

fn mermaid_to_graph_properties(
    mermaid: &str,
    style_type: &str,
    config: &Config,
) -> Result<GraphProperties, String> {
    let newline_re = Regex::new(r"\n|\\n").unwrap();
    let raw_lines: Vec<String> = newline_re
        .split(mermaid)
        .map(|s| s.to_string())
        .collect();

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
        style_classes: HashMap::new(),
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
            ))
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
    fn parse_string(&mut self, line: &str) -> Result<Vec<TextNode>, String> {
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
            let left_nodes = self.parse_string(lhs).unwrap_or_else(|_| vec![parse_node(lhs)]);
            let right_nodes = self.parse_string(rhs).unwrap_or_else(|_| vec![parse_node(rhs)]);
            return Ok(set_arrow(&left_nodes, &right_nodes, &mut self.data));
        }

        if let Some(caps) = label_re.captures(line) {
            let lhs = caps.get(1).unwrap().as_str();
            let label = caps.get(2).unwrap().as_str();
            let rhs = caps.get(3).unwrap().as_str();
            let left_nodes = self.parse_string(lhs).unwrap_or_else(|_| vec![parse_node(lhs)]);
            let right_nodes = self.parse_string(rhs).unwrap_or_else(|_| vec![parse_node(rhs)]);
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
            let left_nodes = self.parse_string(lhs).unwrap_or_else(|_| vec![parse_node(lhs)]);
            let right_nodes = self.parse_string(rhs).unwrap_or_else(|_| vec![parse_node(rhs)]);
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
    let mut style_map = HashMap::new();
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
    debug!("Setting arrow from {:?} to {:?} with label {}", lhs, rhs, label);
    for l in lhs {
        for r in rhs {
            set_data(l, TextEdge {
                parent: l.clone(),
                child: r.clone(),
                label: label.to_string(),
            }, data);
        }
    }
    rhs.to_vec()
}

fn set_arrow(lhs: &[TextNode], rhs: &[TextNode], data: &mut IndexMap<String, Vec<TextEdge>>) -> Vec<TextNode> {
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

fn draw_map(properties: &GraphProperties, show_coords: bool) -> Result<String, String> {
    let mut graph = mk_graph(properties);
    graph.set_style_classes(properties);
    graph.padding_x = properties.padding_x;
    graph.padding_y = properties.padding_y;
    graph.box_border_padding = properties.box_border_padding;
    graph.use_ascii = properties.use_ascii;
    graph.graph_direction = properties.graph_direction.clone();
    graph.set_subgraphs(&properties.subgraphs);
    graph.create_mapping();
    let mut drawing = graph.draw();
    if show_coords {
        drawing = debug_drawing_wrapper(&drawing);
        drawing = debug_coord_wrapper(&drawing, &graph);
    }
    Ok(drawing_to_string(&drawing))
}

fn mk_graph(properties: &GraphProperties) -> Graph {
    let mut graph = Graph {
        nodes: Vec::new(),
        edges: Vec::new(),
        drawing: mk_drawing(0, 0),
        grid: HashMap::new(),
        column_width: HashMap::new(),
        row_height: HashMap::new(),
        style_classes: HashMap::new(),
        style_type: properties.style_type.clone(),
        padding_x: properties.padding_x,
        padding_y: properties.padding_y,
        box_border_padding: properties.box_border_padding,
        subgraphs: Vec::new(),
        offset_x: 0,
        offset_y: 0,
        use_ascii: properties.use_ascii,
        graph_direction: properties.graph_direction.clone(),
        node_index_by_name: HashMap::new(),
    };

    for (node_name, children) in &properties.data {
        let (parent_idx, _) = graph.get_or_insert_node(node_name, "");
        for edge in children {
            let (child_idx, inserted) =
                graph.get_or_insert_node(&edge.child.name, &edge.get_child_style());
            if inserted {
                graph.nodes[parent_idx].style_class_name = edge.parent.style_class.clone();
            }
            graph.edges.push(Edge {
                from: parent_idx,
                to: child_idx,
                text: edge.label.clone(),
                path: Vec::new(),
                label_line: Vec::new(),
                start_dir: MIDDLE,
                end_dir: MIDDLE,
            });
        }
    }

    graph
}

impl TextEdge {
    fn get_child_style(&self) -> String {
        self.child.style_class.clone()
    }
}

impl Graph {
    fn get_or_insert_node(&mut self, name: &str, style_class: &str) -> (usize, bool) {
        if let Some(idx) = self.node_index_by_name.get(name) {
            return (*idx, false);
        }
        let idx = self.nodes.len();
        self.nodes.push(Node {
            name: name.to_string(),
            drawing: None,
            drawing_coord: None,
            grid_coord: None,
            drawn: false,
            index: idx,
            style_class_name: style_class.to_string(),
            style_class: StyleClass::default(),
        });
        self.node_index_by_name.insert(name.to_string(), idx);
        (idx, true)
    }

    fn set_style_classes(&mut self, properties: &GraphProperties) {
        self.style_classes = properties.style_classes.clone();
        self.style_type = properties.style_type.clone();
        self.padding_x = properties.padding_x;
        self.padding_y = properties.padding_y;
        for node in &mut self.nodes {
            if !node.style_class_name.is_empty() {
                if let Some(class) = self.style_classes.get(&node.style_class_name) {
                    node.style_class = class.clone();
                }
            }
        }
    }

    fn set_subgraphs(&mut self, text_subgraphs: &[TextSubgraph]) {
        self.subgraphs = Vec::new();
        for tsg in text_subgraphs {
            let mut nodes = Vec::new();
            for name in &tsg.nodes {
                if let Some(idx) = self.node_index_by_name.get(name) {
                    nodes.push(*idx);
                }
            }
            self.subgraphs.push(Subgraph {
                name: tsg.name.clone(),
                nodes,
                parent: None,
                children: Vec::new(),
                min_x: 0,
                min_y: 0,
                max_x: 0,
                max_y: 0,
            });
        }

        for (idx, tsg) in text_subgraphs.iter().enumerate() {
            if let Some(parent_idx) = tsg.parent {
                self.subgraphs[idx].parent = Some(parent_idx);
            }
            self.subgraphs[idx].children = tsg.children.clone();
        }
    }

    fn create_mapping(&mut self) {
        let mut highest_position_per_level = vec![0; 100];

        let mut nodes_found: HashSet<String> = HashSet::new();
        let mut root_nodes: Vec<usize> = Vec::new();
        for node in &self.nodes {
            if !nodes_found.contains(&node.name) {
                root_nodes.push(node.index);
            }
            nodes_found.insert(node.name.clone());
            for child in self.get_children(node.index) {
                nodes_found.insert(self.nodes[child].name.clone());
            }
        }

        let mut has_external_roots = false;
        let mut has_subgraph_roots_with_edges = false;
        for idx in &root_nodes {
            if self.is_node_in_any_subgraph(*idx) {
                if !self.get_children(*idx).is_empty() {
                    has_subgraph_roots_with_edges = true;
                }
            } else {
                has_external_roots = true;
            }
        }

        let should_separate = self.graph_direction == "LR" && has_external_roots && has_subgraph_roots_with_edges;
        let mut external_root_nodes = Vec::new();
        let mut subgraph_root_nodes = Vec::new();
        if should_separate {
            for idx in &root_nodes {
                if self.is_node_in_any_subgraph(*idx) {
                    subgraph_root_nodes.push(*idx);
                } else {
                    external_root_nodes.push(*idx);
                }
            }
        } else {
            external_root_nodes = root_nodes.clone();
        }

        for idx in &external_root_nodes {
            let coord = if self.graph_direction == "LR" {
                self.reserve_spot_in_grid(*idx, GridCoord { x: 0, y: highest_position_per_level[0] })
            } else {
                self.reserve_spot_in_grid(*idx, GridCoord { x: highest_position_per_level[0], y: 0 })
            };
            self.nodes[*idx].grid_coord = Some(coord);
            highest_position_per_level[0] += 4;
        }

        if should_separate && !subgraph_root_nodes.is_empty() {
            let subgraph_level = 4;
            for idx in &subgraph_root_nodes {
                let coord = if self.graph_direction == "LR" {
                    self.reserve_spot_in_grid(
                        *idx,
                        GridCoord {
                            x: subgraph_level,
                            y: highest_position_per_level[subgraph_level as usize],
                        },
                    )
                } else {
                    self.reserve_spot_in_grid(
                        *idx,
                        GridCoord {
                            x: highest_position_per_level[subgraph_level as usize],
                            y: subgraph_level,
                        },
                    )
                };
                self.nodes[*idx].grid_coord = Some(coord);
                highest_position_per_level[subgraph_level as usize] += 4;
            }
        }

        for idx in 0..self.nodes.len() {
            let grid_coord = self.nodes[idx].grid_coord.unwrap();
            let child_level = if self.graph_direction == "LR" {
                grid_coord.x + 4
            } else {
                grid_coord.y + 4
            };
            let mut highest_position = highest_position_per_level[child_level as usize];
            let children = self.get_children(idx);
            for child_idx in children {
                if self.nodes[child_idx].grid_coord.is_some() {
                    continue;
                }
                let coord = if self.graph_direction == "LR" {
                    self.reserve_spot_in_grid(
                        child_idx,
                        GridCoord {
                            x: child_level,
                            y: highest_position,
                        },
                    )
                } else {
                    self.reserve_spot_in_grid(
                        child_idx,
                        GridCoord {
                            x: highest_position,
                            y: child_level,
                        },
                    )
                };
                self.nodes[child_idx].grid_coord = Some(coord);
                highest_position_per_level[child_level as usize] = highest_position + 4;
                highest_position = highest_position_per_level[child_level as usize];
            }
        }

        for idx in 0..self.nodes.len() {
            self.set_column_width(idx);
        }

        for edge_idx in 0..self.edges.len() {
            self.determine_path(edge_idx);
            let path = self.edges[edge_idx].path.clone();
            self.increase_grid_size_for_path(&path);
            self.determine_label_line(edge_idx);
        }

        for idx in 0..self.nodes.len() {
            let dc = self.grid_to_drawing_coord(self.nodes[idx].grid_coord.unwrap(), None);
            self.nodes[idx].drawing_coord = Some(dc);
            let drawing = draw_box(&self.nodes[idx], self);
            self.nodes[idx].drawing = Some(drawing);
        }

        self.set_drawing_size_to_grid_constraints();
        self.calculate_subgraph_bounding_boxes();
        self.offset_drawing_for_subgraphs();
    }

    fn set_column_width(&mut self, idx: usize) {
        let node = &self.nodes[idx];
        let grid_coord = node.grid_coord.unwrap();
        let name_len = node.name.chars().count() as i32;
        let col1 = 1;
        let col2 = 2 * self.box_border_padding + name_len;
        let col3 = 1;
        let cols = [col1, col2, col3];
        let rows = [1, 1 + 2 * self.box_border_padding, 1];

        for (offset, col) in cols.iter().enumerate() {
            let x = grid_coord.x + offset as i32;
            let entry = self.column_width.entry(x).or_insert(0);
            *entry = max(*entry, *col);
        }
        for (offset, row) in rows.iter().enumerate() {
            let y = grid_coord.y + offset as i32;
            let entry = self.row_height.entry(y).or_insert(0);
            *entry = max(*entry, *row);
        }

        if grid_coord.x > 0 {
            self.column_width.insert(grid_coord.x - 1, self.padding_x);
        }
        if grid_coord.y > 0 {
            let mut base_padding = self.padding_y;
            if self.has_incoming_edge_from_outside_subgraph(idx) {
                base_padding += 4;
            }
            let entry = self.row_height.entry(grid_coord.y - 1).or_insert(0);
            *entry = max(*entry, base_padding);
        }
    }

    fn increase_grid_size_for_path(&mut self, path: &[GridCoord]) {
        for coord in path {
            self.column_width.entry(coord.x).or_insert(self.padding_x / 2);
            self.row_height.entry(coord.y).or_insert(self.padding_y / 2);
        }
    }

    fn reserve_spot_in_grid(&mut self, node_idx: usize, requested: GridCoord) -> GridCoord {
        let mut coord = requested;
        loop {
            if !self.grid.contains_key(&coord) {
                break;
            }
            if self.graph_direction == "LR" {
                coord = GridCoord {
                    x: coord.x,
                    y: coord.y + 4,
                };
            } else {
                coord = GridCoord {
                    x: coord.x + 4,
                    y: coord.y,
                };
            }
        }
        for x in 0..3 {
            for y in 0..3 {
                let reserved = GridCoord {
                    x: coord.x + x,
                    y: coord.y + y,
                };
                self.grid.insert(reserved, node_idx);
            }
        }
        coord
    }

    fn get_edges_from_node(&self, node_idx: usize) -> Vec<usize> {
        self.edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.from == node_idx)
            .map(|(idx, _)| idx)
            .collect()
    }

    fn get_children(&self, node_idx: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|edge| edge.from == node_idx)
            .map(|edge| edge.to)
            .collect()
    }

    fn grid_to_drawing_coord(&self, coord: GridCoord, dir: Option<Direction>) -> DrawingCoord {
        let target = if let Some(dir) = dir {
            GridCoord {
                x: coord.x + dir.dx,
                y: coord.y + dir.dy,
            }
        } else {
            coord
        };
        let mut x = 0;
        let mut y = 0;
        for col in 0..target.x {
            x += *self.column_width.get(&col).unwrap_or(&0);
        }
        for row in 0..target.y {
            y += *self.row_height.get(&row).unwrap_or(&0);
        }
        DrawingCoord {
            x: x + self.column_width.get(&target.x).unwrap_or(&0) / 2 + self.offset_x,
            y: y + self.row_height.get(&target.y).unwrap_or(&0) / 2 + self.offset_y,
        }
    }

    fn determine_path(&mut self, edge_idx: usize) {
        let (preferred_dir, preferred_opp, alternative_dir, alternative_opp) =
            determine_start_and_end_dir(self.graph_direction.as_str(), &self.edges[edge_idx], self);

        let from = self.nodes[self.edges[edge_idx].from]
            .grid_coord
            .unwrap()
            .direction(preferred_dir);
        let to = self.nodes[self.edges[edge_idx].to]
            .grid_coord
            .unwrap()
            .direction(preferred_opp);

        let preferred_path = match self.get_path(from, to) {
            Ok(path) => merge_path(path),
            Err(_) => {
                self.edges[edge_idx].start_dir = alternative_dir;
                self.edges[edge_idx].end_dir = alternative_opp;
                self.edges[edge_idx].path = Vec::new();
                return;
            }
        };

        let from_alt = self.nodes[self.edges[edge_idx].from]
            .grid_coord
            .unwrap()
            .direction(alternative_dir);
        let to_alt = self.nodes[self.edges[edge_idx].to]
            .grid_coord
            .unwrap()
            .direction(alternative_opp);

        let alternative_path = match self.get_path(from_alt, to_alt) {
            Ok(path) => merge_path(path),
            Err(_) => {
                self.edges[edge_idx].start_dir = preferred_dir;
                self.edges[edge_idx].end_dir = preferred_opp;
                self.edges[edge_idx].path = preferred_path;
                return;
            }
        };

        if preferred_path.len() <= alternative_path.len() {
            self.edges[edge_idx].start_dir = preferred_dir;
            self.edges[edge_idx].end_dir = preferred_opp;
            self.edges[edge_idx].path = preferred_path;
        } else {
            self.edges[edge_idx].start_dir = alternative_dir;
            self.edges[edge_idx].end_dir = alternative_opp;
            self.edges[edge_idx].path = alternative_path;
        }
    }

    fn determine_label_line(&mut self, edge_idx: usize) {
        let label_len = self.edges[edge_idx].text.chars().count() as i32;
        if label_len == 0 {
            return;
        }
        let path = self.edges[edge_idx].path.clone();
        if path.len() < 2 {
            return;
        }
        let mut prev_step = path[0];
        let mut largest_line = vec![prev_step, path[1]];
        let mut largest_line_size = 0;
        for step in path.iter().skip(1) {
            let line = vec![prev_step, *step];
            let line_width = self.calculate_line_width(&line);
            if line_width >= label_len {
                largest_line = line;
                break;
            } else if line_width > largest_line_size {
                largest_line_size = line_width;
                largest_line = line;
            }
            prev_step = *step;
        }

        let (max_x, min_x) = if largest_line[0].x > largest_line[1].x {
            (largest_line[0].x, largest_line[1].x)
        } else {
            (largest_line[1].x, largest_line[0].x)
        };
        let middle_x = min_x + (max_x - min_x) / 2;
        let entry = self.column_width.entry(middle_x).or_insert(0);
        *entry = max(*entry, label_len + 2);
        self.edges[edge_idx].label_line = largest_line;
    }

    fn calculate_line_width(&self, line: &[GridCoord]) -> i32 {
        line.iter()
            .map(|c| *self.column_width.get(&c.x).unwrap_or(&0))
            .sum()
    }

    fn draw(&mut self) -> Drawing {
        self.draw_subgraphs();
        for idx in 0..self.nodes.len() {
            if !self.nodes[idx].drawn {
                self.draw_node(idx);
            }
        }

        let mut line_drawings = Vec::new();
        let mut corner_drawings = Vec::new();
        let mut arrow_head_drawings = Vec::new();
        let mut box_start_drawings = Vec::new();
        let mut label_drawings = Vec::new();

        for edge_idx in 0..self.edges.len() {
            let (line, box_start, arrow_head, corners, label) = self.draw_edge(edge_idx);
            line_drawings.push(line);
            corner_drawings.push(corners);
            arrow_head_drawings.push(arrow_head);
            box_start_drawings.push(box_start);
            label_drawings.push(label);
        }

        self.drawing = self.merge_drawings(&self.drawing, DrawingCoord { x: 0, y: 0 }, &line_drawings);
        self.drawing = self.merge_drawings(&self.drawing, DrawingCoord { x: 0, y: 0 }, &corner_drawings);
        self.drawing = self.merge_drawings(&self.drawing, DrawingCoord { x: 0, y: 0 }, &arrow_head_drawings);
        self.drawing = self.merge_drawings(&self.drawing, DrawingCoord { x: 0, y: 0 }, &box_start_drawings);
        self.drawing = self.merge_drawings(&self.drawing, DrawingCoord { x: 0, y: 0 }, &label_drawings);

        self.draw_subgraph_labels();

        self.drawing.clone()
    }

    fn draw_node(&mut self, idx: usize) {
        if let Some(coord) = self.nodes[idx].drawing_coord {
            if let Some(drawing) = &self.nodes[idx].drawing {
                self.drawing = self.merge_drawings(&self.drawing, coord, &[drawing.clone()]);
                self.nodes[idx].drawn = true;
            }
        }
    }

    fn draw_edge(&self, edge_idx: usize) -> (Drawing, Drawing, Drawing, Drawing, Drawing) {
        let edge = &self.edges[edge_idx];
        if edge.path.is_empty() {
            return (mk_drawing(0, 0), mk_drawing(0, 0), mk_drawing(0, 0), mk_drawing(0, 0), mk_drawing(0, 0));
        }
        let from = self.nodes[edge.from].grid_coord.unwrap().direction(edge.start_dir);
        let to = self.nodes[edge.to].grid_coord.unwrap().direction(edge.end_dir);
        self.draw_arrow(from, to, edge)
    }

    fn draw_subgraphs(&mut self) {
        let sorted = self.sort_subgraphs_by_depth();
        for idx in sorted {
            let sg = &self.subgraphs[idx];
            if sg.nodes.is_empty() {
                continue;
            }
            let drawing = draw_subgraph(sg, self);
            let offset = DrawingCoord {
                x: sg.min_x,
                y: sg.min_y,
            };
            self.drawing = self.merge_drawings(&self.drawing, offset, &[drawing]);
        }
    }

    fn draw_subgraph_labels(&mut self) {
        for sg in &self.subgraphs {
            if sg.nodes.is_empty() {
                continue;
            }
            let (label, offset) = draw_subgraph_label(sg);
            self.drawing = self.merge_drawings(&self.drawing, offset, &[label]);
        }
    }

    fn sort_subgraphs_by_depth(&self) -> Vec<usize> {
        let mut depths = HashMap::new();
        for (idx, _) in self.subgraphs.iter().enumerate() {
            let depth = self.get_subgraph_depth(idx);
            depths.insert(idx, depth);
        }
        let mut sorted: Vec<usize> = (0..self.subgraphs.len()).collect();
        sorted.sort_by_key(|idx| depths.get(idx).copied().unwrap_or(0));
        sorted
    }

    fn get_subgraph_depth(&self, idx: usize) -> i32 {
        if let Some(parent) = self.subgraphs[idx].parent {
            1 + self.get_subgraph_depth(parent)
        } else {
            0
        }
    }

    fn calculate_subgraph_bounding_boxes(&mut self) {
        for idx in 0..self.subgraphs.len() {
            self.calculate_subgraph_bounding_box(idx);
        }
        self.ensure_subgraph_spacing();
    }

    fn calculate_subgraph_bounding_box(&mut self, idx: usize) {
        if self.subgraphs[idx].nodes.is_empty() {
            return;
        }
        let mut min_x = 1_000_000;
        let mut min_y = 1_000_000;
        let mut max_x = -1_000_000;
        let mut max_y = -1_000_000;

        let children = self.subgraphs[idx].children.clone();
        for child_idx in children {
            self.calculate_subgraph_bounding_box(child_idx);
            if !self.subgraphs[child_idx].nodes.is_empty() {
                min_x = min(min_x, self.subgraphs[child_idx].min_x);
                min_y = min(min_y, self.subgraphs[child_idx].min_y);
                max_x = max(max_x, self.subgraphs[child_idx].max_x);
                max_y = max(max_y, self.subgraphs[child_idx].max_y);
            }
        }

        let nodes = self.subgraphs[idx].nodes.clone();
        for node_idx in nodes {
            let node = &self.nodes[node_idx];
            if node.drawing_coord.is_none() || node.drawing.is_none() {
                continue;
            }
            let coord = node.drawing_coord.unwrap();
            let drawing = node.drawing.as_ref().unwrap();
            let node_min_x = coord.x;
            let node_min_y = coord.y;
            let node_max_x = node_min_x + drawing.len() as i32 - 1;
            let node_max_y = node_min_y + drawing[0].len() as i32 - 1;
            min_x = min(min_x, node_min_x);
            min_y = min(min_y, node_min_y);
            max_x = max(max_x, node_max_x);
            max_y = max(max_y, node_max_y);
        }

        let subgraph_padding = 2;
        let subgraph_label_space = 2;
        self.subgraphs[idx].min_x = min_x - subgraph_padding;
        self.subgraphs[idx].min_y = min_y - subgraph_padding - subgraph_label_space;
        self.subgraphs[idx].max_x = max_x + subgraph_padding;
        self.subgraphs[idx].max_y = max_y + subgraph_padding;
    }

    fn ensure_subgraph_spacing(&mut self) {
        let min_spacing = 1;
        let root_subgraphs: Vec<usize> = self
            .subgraphs
            .iter()
            .enumerate()
            .filter(|(_, sg)| sg.parent.is_none() && !sg.nodes.is_empty())
            .map(|(idx, _)| idx)
            .collect();

        for i in 0..root_subgraphs.len() {
            for j in (i + 1)..root_subgraphs.len() {
                let (sg1_idx, sg2_idx) = (root_subgraphs[i], root_subgraphs[j]);
                let (sg1, sg2) = {
                    let (a, b) = self.subgraphs.split_at_mut(sg2_idx.max(sg1_idx));
                    if sg1_idx < sg2_idx {
                        (&mut a[sg1_idx], &mut b[0])
                    } else {
                        (&mut b[0], &mut a[sg2_idx])
                    }
                };

                if sg1.min_x < sg2.max_x && sg1.max_x > sg2.min_x {
                    if sg1.max_y >= sg2.min_y - min_spacing && sg1.min_y < sg2.min_y {
                        sg2.min_y = sg1.max_y + min_spacing + 1;
                    } else if sg2.max_y >= sg1.min_y - min_spacing && sg2.min_y < sg1.min_y {
                        sg1.min_y = sg2.max_y + min_spacing + 1;
                    }
                }

                if sg1.min_y < sg2.max_y && sg1.max_y > sg2.min_y {
                    if sg1.max_x >= sg2.min_x - min_spacing && sg1.min_x < sg2.min_x {
                        sg2.min_x = sg1.max_x + min_spacing + 1;
                    } else if sg2.max_x >= sg1.min_x - min_spacing && sg2.min_x < sg1.min_x {
                        sg1.min_x = sg2.max_x + min_spacing + 1;
                    }
                }
            }
        }
    }

    fn offset_drawing_for_subgraphs(&mut self) {
        if self.subgraphs.is_empty() {
            return;
        }
        let mut min_x = 0;
        let mut min_y = 0;
        for sg in &self.subgraphs {
            min_x = min(min_x, sg.min_x);
            min_y = min(min_y, sg.min_y);
        }

        let offset_x = -min_x;
        let offset_y = -min_y;
        if offset_x == 0 && offset_y == 0 {
            return;
        }

        self.offset_x = offset_x;
        self.offset_y = offset_y;

        for sg in &mut self.subgraphs {
            sg.min_x += offset_x;
            sg.min_y += offset_y;
            sg.max_x += offset_x;
            sg.max_y += offset_y;
        }

        for node in &mut self.nodes {
            if let Some(coord) = &mut node.drawing_coord {
                coord.x += offset_x;
                coord.y += offset_y;
            }
        }
    }

    fn is_node_in_any_subgraph(&self, node_idx: usize) -> bool {
        self.subgraphs
            .iter()
            .any(|sg| sg.nodes.iter().any(|idx| *idx == node_idx))
    }

    fn get_node_subgraph(&self, node_idx: usize) -> Option<usize> {
        self.subgraphs.iter().position(|sg| sg.nodes.iter().any(|idx| *idx == node_idx))
    }

    fn has_incoming_edge_from_outside_subgraph(&self, node_idx: usize) -> bool {
        let node_subgraph = match self.get_node_subgraph(node_idx) {
            Some(idx) => idx,
            None => return false,
        };

        let mut has_external_edge = false;
        for edge in &self.edges {
            if edge.to == node_idx {
                let source_subgraph = self.get_node_subgraph(edge.from);
                if source_subgraph != Some(node_subgraph) {
                    has_external_edge = true;
                    break;
                }
            }
        }
        if !has_external_edge {
            return false;
        }

        for other in &self.subgraphs[node_subgraph].nodes {
            if *other == node_idx {
                continue;
            }
            let other_coord = self.nodes[*other].grid_coord;
            if other_coord.is_none() {
                continue;
            }
            let mut other_has_external = false;
            for edge in &self.edges {
                if edge.to == *other {
                    let source_subgraph = self.get_node_subgraph(edge.from);
                    if source_subgraph != Some(node_subgraph) {
                        other_has_external = true;
                        break;
                    }
                }
            }
            if other_has_external {
                if let (Some(other_coord), Some(node_coord)) = (other_coord, self.nodes[node_idx].grid_coord) {
                    if other_coord.y < node_coord.y {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn set_drawing_size_to_grid_constraints(&mut self) {
        let max_x: i32 = self.column_width.values().sum();
        let max_y: i32 = self.row_height.values().sum();
        let drawing = &mut self.drawing;
        increase_size(drawing, max_x - 1, max_y - 1);
    }

    fn draw_arrow(&self, from: GridCoord, to: GridCoord, edge: &Edge) -> (Drawing, Drawing, Drawing, Drawing, Drawing) {
        if edge.path.is_empty() {
            return (mk_drawing(0, 0), mk_drawing(0, 0), mk_drawing(0, 0), mk_drawing(0, 0), mk_drawing(0, 0));
        }
        let label = self.draw_arrow_label(edge);
        let (path, lines_drawn, line_dirs) = self.draw_path(&edge.path);
        let box_start = self.draw_box_start(&edge.path, &lines_drawn[0]);
        let arrow_head = self.draw_arrow_head(lines_drawn.last().unwrap(), *line_dirs.last().unwrap());
        let corners = self.draw_corners(&edge.path);
        (path, box_start, arrow_head, corners, label)
    }

    fn draw_path(&self, path: &[GridCoord]) -> (Drawing, Vec<Vec<DrawingCoord>>, Vec<Direction>) {
        let mut drawing = copy_canvas(&self.drawing);
        let mut lines_drawn = Vec::new();
        let mut line_dirs = Vec::new();
        let mut previous = path[0];
        for next in path.iter().skip(1) {
            let prev_dc = self.grid_to_drawing_coord(previous, None);
            let next_dc = self.grid_to_drawing_coord(*next, None);
            if prev_dc.equals(next_dc) {
                previous = *next;
                continue;
            }
            let dir = determine_direction(GenericCoord { x: previous.x, y: previous.y }, GenericCoord { x: next.x, y: next.y });
            let mut line = self.draw_line(&mut drawing, prev_dc, next_dc, 1, -1);
            if line.is_empty() {
                line.push(prev_dc);
            }
            lines_drawn.push(line);
            line_dirs.push(dir);
            previous = *next;
        }
        (drawing, lines_drawn, line_dirs)
    }

    fn draw_line(
        &self,
        drawing: &mut Drawing,
        from: DrawingCoord,
        to: DrawingCoord,
        offset_from: i32,
        offset_to: i32,
    ) -> Vec<DrawingCoord> {
        let dir = determine_direction(GenericCoord { x: from.x, y: from.y }, GenericCoord { x: to.x, y: to.y });
        let mut drawn = Vec::new();
        if !self.use_ascii {
            match dir {
                d if d == UP => {
                    for y in (to.y - offset_to)..=(from.y - offset_from) {
                        drawn.push(DrawingCoord { x: from.x, y });
                        set_cell(drawing, from.x, y, "│");
                    }
                }
                d if d == DOWN => {
                    for y in (from.y + offset_from)..=(to.y + offset_to) {
                        drawn.push(DrawingCoord { x: from.x, y });
                        set_cell(drawing, from.x, y, "│");
                    }
                }
                d if d == LEFT => {
                    for x in (to.x - offset_to)..=(from.x - offset_from) {
                        drawn.push(DrawingCoord { x, y: from.y });
                        set_cell(drawing, x, from.y, "─");
                    }
                }
                d if d == RIGHT => {
                    for x in (from.x + offset_from)..=(to.x + offset_to) {
                        drawn.push(DrawingCoord { x, y: from.y });
                        set_cell(drawing, x, from.y, "─");
                    }
                }
                d if d == UPPER_LEFT => {
                    let mut x = from.x;
                    let mut y = from.y - offset_from;
                    while x >= to.x - offset_to && y >= to.y - offset_to {
                        drawn.push(DrawingCoord { x, y });
                        set_cell(drawing, x, y, "╲");
                        x -= 1;
                        y -= 1;
                    }
                }
                d if d == UPPER_RIGHT => {
                    let mut x = from.x;
                    let mut y = from.y - offset_from;
                    while x <= to.x + offset_to && y >= to.y - offset_to {
                        drawn.push(DrawingCoord { x, y });
                        set_cell(drawing, x, y, "╱");
                        x += 1;
                        y -= 1;
                    }
                }
                d if d == LOWER_LEFT => {
                    let mut x = from.x;
                    let mut y = from.y + offset_from;
                    while x >= to.x - offset_to && y <= to.y + offset_to {
                        drawn.push(DrawingCoord { x, y });
                        set_cell(drawing, x, y, "╱");
                        x -= 1;
                        y += 1;
                    }
                }
                d if d == LOWER_RIGHT => {
                    let mut x = from.x;
                    let mut y = from.y + offset_from;
                    while x <= to.x + offset_to && y <= to.y + offset_to {
                        drawn.push(DrawingCoord { x, y });
                        set_cell(drawing, x, y, "╲");
                        x += 1;
                        y += 1;
                    }
                }
                _ => {}
            }
        } else {
            match dir {
                d if d == UP => {
                    for y in (to.y - offset_to)..=(from.y - offset_from) {
                        drawn.push(DrawingCoord { x: from.x, y });
                        set_cell(drawing, from.x, y, "|");
                    }
                }
                d if d == DOWN => {
                    for y in (from.y + offset_from)..=(to.y + offset_to) {
                        drawn.push(DrawingCoord { x: from.x, y });
                        set_cell(drawing, from.x, y, "|");
                    }
                }
                d if d == LEFT => {
                    for x in (to.x - offset_to)..=(from.x - offset_from) {
                        drawn.push(DrawingCoord { x, y: from.y });
                        set_cell(drawing, x, from.y, "-");
                    }
                }
                d if d == RIGHT => {
                    for x in (from.x + offset_from)..=(to.x + offset_to) {
                        drawn.push(DrawingCoord { x, y: from.y });
                        set_cell(drawing, x, from.y, "-");
                    }
                }
                d if d == UPPER_LEFT => {
                    let mut x = from.x;
                    let mut y = from.y - offset_from;
                    while x >= to.x - offset_to && y >= to.y - offset_to {
                        drawn.push(DrawingCoord { x, y });
                        set_cell(drawing, x, y, "\\");
                        x -= 1;
                        y -= 1;
                    }
                }
                d if d == UPPER_RIGHT => {
                    let mut x = from.x;
                    let mut y = from.y - offset_from;
                    while x <= to.x + offset_to && y >= to.y - offset_to {
                        drawn.push(DrawingCoord { x, y });
                        set_cell(drawing, x, y, "/");
                        x += 1;
                        y -= 1;
                    }
                }
                d if d == LOWER_LEFT => {
                    let mut x = from.x;
                    let mut y = from.y + offset_from;
                    while x >= to.x - offset_to && y <= to.y + offset_to {
                        drawn.push(DrawingCoord { x, y });
                        set_cell(drawing, x, y, "/");
                        x -= 1;
                        y += 1;
                    }
                }
                d if d == LOWER_RIGHT => {
                    let mut x = from.x;
                    let mut y = from.y + offset_from;
                    while x <= to.x + offset_to && y <= to.y + offset_to {
                        drawn.push(DrawingCoord { x, y });
                        set_cell(drawing, x, y, "\\");
                        x += 1;
                        y += 1;
                    }
                }
                _ => {}
            }
        }
        drawn
    }

    fn draw_box_start(&self, path: &[GridCoord], first_line: &[DrawingCoord]) -> Drawing {
        let mut drawing = copy_canvas(&self.drawing);
        if self.use_ascii || first_line.is_empty() {
            return drawing;
        }
        let from = first_line[0];
        let dir = determine_direction(
            GenericCoord { x: path[0].x, y: path[0].y },
            GenericCoord { x: path[1].x, y: path[1].y },
        );
        match dir {
            d if d == UP => set_cell(&mut drawing, from.x, from.y + 1, "┴"),
            d if d == DOWN => set_cell(&mut drawing, from.x, from.y - 1, "┬"),
            d if d == LEFT => set_cell(&mut drawing, from.x + 1, from.y, "┤"),
            d if d == RIGHT => set_cell(&mut drawing, from.x - 1, from.y, "├"),
            _ => {}
        }
        drawing
    }

    fn draw_arrow_head(&self, line: &[DrawingCoord], fallback: Direction) -> Drawing {
        let mut drawing = copy_canvas(&self.drawing);
        if line.is_empty() {
            return drawing;
        }
        let from = line[0];
        let last = line[line.len() - 1];
        let mut dir = determine_direction(GenericCoord { x: from.x, y: from.y }, GenericCoord { x: last.x, y: last.y });
        if line.len() == 1 || dir == MIDDLE {
            dir = fallback;
        }

        let ch = if !self.use_ascii {
            match dir {
                d if d == UP => "▲",
                d if d == DOWN => "▼",
                d if d == LEFT => "◄",
                d if d == RIGHT => "►",
                d if d == UPPER_RIGHT => "◥",
                d if d == UPPER_LEFT => "◤",
                d if d == LOWER_RIGHT => "◢",
                d if d == LOWER_LEFT => "◣",
                _ => match fallback {
                    d if d == UP => "▲",
                    d if d == DOWN => "▼",
                    d if d == LEFT => "◄",
                    d if d == RIGHT => "►",
                    d if d == UPPER_RIGHT => "◥",
                    d if d == UPPER_LEFT => "◤",
                    d if d == LOWER_RIGHT => "◢",
                    d if d == LOWER_LEFT => "◣",
                    _ => "●",
                },
            }
        } else {
            match dir {
                d if d == UP => "^",
                d if d == DOWN => "v",
                d if d == LEFT => "<",
                d if d == RIGHT => ">",
                _ => match fallback {
                    d if d == UP => "^",
                    d if d == DOWN => "v",
                    d if d == LEFT => "<",
                    d if d == RIGHT => ">",
                    _ => "*",
                },
            }
        };

        set_cell(&mut drawing, last.x, last.y, ch);
        drawing
    }

    fn draw_corners(&self, path: &[GridCoord]) -> Drawing {
        let mut drawing = copy_canvas(&self.drawing);
        for idx in 1..path.len().saturating_sub(1) {
            let coord = path[idx];
            let drawing_coord = self.grid_to_drawing_coord(coord, None);
            let prev_dir = determine_direction(
                GenericCoord { x: path[idx - 1].x, y: path[idx - 1].y },
                GenericCoord { x: coord.x, y: coord.y },
            );
            let next_dir = determine_direction(
                GenericCoord { x: coord.x, y: coord.y },
                GenericCoord { x: path[idx + 1].x, y: path[idx + 1].y },
            );
            let corner = if !self.use_ascii {
                if (prev_dir == RIGHT && next_dir == DOWN) || (prev_dir == UP && next_dir == LEFT) {
                    "┐"
                } else if (prev_dir == RIGHT && next_dir == UP) || (prev_dir == DOWN && next_dir == LEFT) {
                    "┘"
                } else if (prev_dir == LEFT && next_dir == DOWN) || (prev_dir == UP && next_dir == RIGHT) {
                    "┌"
                } else if (prev_dir == LEFT && next_dir == UP) || (prev_dir == DOWN && next_dir == RIGHT) {
                    "└"
                } else {
                    "+"
                }
            } else {
                "+"
            };
            set_cell(&mut drawing, drawing_coord.x, drawing_coord.y, corner);
        }
        drawing
    }

    fn draw_arrow_label(&self, edge: &Edge) -> Drawing {
        let mut drawing = copy_canvas(&self.drawing);
        if edge.text.is_empty() || edge.label_line.len() < 2 {
            return drawing;
        }
        let line = self.line_to_drawing(&edge.label_line);
        draw_text_on_line(&mut drawing, &line, &edge.text);
        drawing
    }

    fn line_to_drawing(&self, line: &[GridCoord]) -> Vec<DrawingCoord> {
        line.iter()
            .map(|coord| self.grid_to_drawing_coord(*coord, None))
            .collect()
    }

    fn get_path(&self, from: GridCoord, to: GridCoord) -> Result<Vec<GridCoord>, String> {
        let mut pq = BinaryHeap::new();
        pq.push(QueueItem { coord: from, priority: 0 });
        let mut cost_so_far: HashMap<GridCoord, i32> = HashMap::new();
        let mut came_from: HashMap<GridCoord, Option<GridCoord>> = HashMap::new();
        cost_so_far.insert(from, 0);
        came_from.insert(from, None);

        let directions = [
            GridCoord { x: 1, y: 0 },
            GridCoord { x: -1, y: 0 },
            GridCoord { x: 0, y: 1 },
            GridCoord { x: 0, y: -1 },
        ];

        while let Some(current) = pq.pop().map(|item| item.coord) {
            if current.equals(to) {
                let mut path = Vec::new();
                let mut c = Some(current);
                while let Some(coord) = c {
                    path.insert(0, coord);
                    c = came_from.get(&coord).and_then(|v| *v);
                }
                return Ok(path);
            }

            for dir in &directions {
                let next = GridCoord {
                    x: current.x + dir.x,
                    y: current.y + dir.y,
                };
                if !self.is_free_in_grid(next) && !next.equals(to) {
                    continue;
                }
                let new_cost = cost_so_far.get(&current).unwrap_or(&0) + 1;
                if !cost_so_far.contains_key(&next) || new_cost < *cost_so_far.get(&next).unwrap() {
                    cost_so_far.insert(next, new_cost);
                    let priority = new_cost + heuristic(next, to);
                    pq.push(QueueItem { coord: next, priority });
                    came_from.insert(next, Some(current));
                }
            }
        }

        Err("no path found".to_string())
    }

    fn is_free_in_grid(&self, coord: GridCoord) -> bool {
        if coord.x < 0 || coord.y < 0 {
            return false;
        }
        !self.grid.contains_key(&coord)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct QueueItem {
    coord: GridCoord,
    priority: i32,
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn heuristic(a: GridCoord, b: GridCoord) -> i32 {
    let abs_x = (a.x - b.x).abs();
    let abs_y = (a.y - b.y).abs();
    if abs_x == 0 || abs_y == 0 {
        abs_x + abs_y
    } else {
        abs_x + abs_y + 1
    }
}

fn merge_path(path: Vec<GridCoord>) -> Vec<GridCoord> {
    if path.len() <= 2 {
        return path;
    }
    let mut remove: HashSet<usize> = HashSet::new();
    let mut step0 = path[0];
    let mut step1 = path[1];
    for (idx, step2) in path.iter().skip(2).enumerate() {
        let prev_dir = determine_direction(GenericCoord { x: step0.x, y: step0.y }, GenericCoord { x: step1.x, y: step1.y });
        let dir = determine_direction(GenericCoord { x: step1.x, y: step1.y }, GenericCoord { x: step2.x, y: step2.y });
        if prev_dir == dir {
            remove.insert(idx + 1);
        }
        step0 = step1;
        step1 = *step2;
    }
    path.into_iter()
        .enumerate()
        .filter(|(idx, _)| !remove.contains(idx))
        .map(|(_, coord)| coord)
        .collect()
}

fn determine_direction(from: GenericCoord, to: GenericCoord) -> Direction {
    if from.x == to.x {
        if from.y < to.y {
            DOWN
        } else {
            UP
        }
    } else if from.y == to.y {
        if from.x < to.x {
            RIGHT
        } else {
            LEFT
        }
    } else if from.x < to.x {
        if from.y < to.y {
            LOWER_RIGHT
        } else {
            UPPER_RIGHT
        }
    } else if from.y < to.y {
        LOWER_LEFT
    } else {
        UPPER_LEFT
    }
}

fn self_reference_direction(graph_direction: &str) -> (Direction, Direction, Direction, Direction) {
    if graph_direction == "LR" {
        (RIGHT, DOWN, DOWN, RIGHT)
    } else {
        (DOWN, RIGHT, RIGHT, DOWN)
    }
}

fn determine_start_and_end_dir(
    graph_direction: &str,
    edge: &Edge,
    graph: &Graph,
) -> (Direction, Direction, Direction, Direction) {
    if edge.from == edge.to {
        return self_reference_direction(graph_direction);
    }
    let from_coord = graph.nodes[edge.from].grid_coord.unwrap();
    let to_coord = graph.nodes[edge.to].grid_coord.unwrap();
    let d = determine_direction(
        GenericCoord { x: from_coord.x, y: from_coord.y },
        GenericCoord { x: to_coord.x, y: to_coord.y },
    );
    let is_backwards = if graph_direction == "LR" {
        d == LEFT || d == UPPER_LEFT || d == LOWER_LEFT
    } else {
        d == UP || d == UPPER_LEFT || d == UPPER_RIGHT
    };

    let (mut preferred_dir, mut preferred_opp, mut alt_dir, mut alt_opp) = (d, d.opposite(), d, d.opposite());
    match d {
        dir if dir == LOWER_RIGHT => {
            if graph_direction == "LR" {
                preferred_dir = DOWN;
                preferred_opp = LEFT;
                alt_dir = RIGHT;
                alt_opp = UP;
            } else {
                preferred_dir = RIGHT;
                preferred_opp = UP;
                alt_dir = DOWN;
                alt_opp = LEFT;
            }
        }
        dir if dir == UPPER_RIGHT => {
            if graph_direction == "LR" {
                preferred_dir = UP;
                preferred_opp = LEFT;
                alt_dir = RIGHT;
                alt_opp = DOWN;
            } else {
                preferred_dir = RIGHT;
                preferred_opp = DOWN;
                alt_dir = UP;
                alt_opp = LEFT;
            }
        }
        dir if dir == LOWER_LEFT => {
            if graph_direction == "LR" {
                preferred_dir = DOWN;
                preferred_opp = DOWN;
                alt_dir = LEFT;
                alt_opp = UP;
            } else {
                preferred_dir = LEFT;
                preferred_opp = UP;
                alt_dir = DOWN;
                alt_opp = RIGHT;
            }
        }
        dir if dir == UPPER_LEFT => {
            if graph_direction == "LR" {
                preferred_dir = DOWN;
                preferred_opp = DOWN;
                alt_dir = LEFT;
                alt_opp = DOWN;
            } else {
                preferred_dir = RIGHT;
                preferred_opp = RIGHT;
                alt_dir = UP;
                alt_opp = RIGHT;
            }
        }
        _ => {
            if is_backwards {
                if graph_direction == "LR" && d == LEFT {
                    preferred_dir = DOWN;
                    preferred_opp = DOWN;
                    alt_dir = LEFT;
                    alt_opp = RIGHT;
                } else if graph_direction == "TD" && d == UP {
                    preferred_dir = RIGHT;
                    preferred_opp = RIGHT;
                    alt_dir = UP;
                    alt_opp = DOWN;
                } else {
                    preferred_dir = d;
                    preferred_opp = d.opposite();
                    alt_dir = d;
                    alt_opp = d.opposite();
                }
            }
        }
    }

    (preferred_dir, preferred_opp, alt_dir, alt_opp)
}

fn draw_box(node: &Node, graph: &Graph) -> Drawing {
    let grid = node.grid_coord.unwrap();
    let mut w = 0;
    let mut h = 0;
    for i in 0..2 {
        w += graph.column_width.get(&(grid.x + i)).unwrap_or(&0);
        h += graph.row_height.get(&(grid.y + i)).unwrap_or(&0);
    }
    let mut drawing = mk_drawing(w, h);
    if !graph.use_ascii {
        for x in 1..w {
            set_cell(&mut drawing, x, 0, "─");
            set_cell(&mut drawing, x, h, "─");
        }
        for y in 1..h {
            set_cell(&mut drawing, 0, y, "│");
            set_cell(&mut drawing, w, y, "│");
        }
        set_cell(&mut drawing, 0, 0, "┌");
        set_cell(&mut drawing, w, 0, "┐");
        set_cell(&mut drawing, 0, h, "└");
        set_cell(&mut drawing, w, h, "┘");
    } else {
        for x in 1..w {
            set_cell(&mut drawing, x, 0, "-");
            set_cell(&mut drawing, x, h, "-");
        }
        for y in 1..h {
            set_cell(&mut drawing, 0, y, "|");
            set_cell(&mut drawing, w, y, "|");
        }
        set_cell(&mut drawing, 0, 0, "+");
        set_cell(&mut drawing, w, 0, "+");
        set_cell(&mut drawing, 0, h, "+");
        set_cell(&mut drawing, w, h, "+");
    }

    let text_y = h / 2;
    let name_len = node.name.chars().count() as i32;
    let text_x = w / 2 - ceil_div(name_len, 2) + 1;
    for (i, ch) in node.name.chars().enumerate() {
        let wrapped = wrap_text_in_color(ch.to_string(), node.style_class.styles.get("color"), &graph.style_type);
        set_cell(&mut drawing, text_x + i as i32, text_y, &wrapped);
    }
    drawing
}

fn draw_subgraph(sg: &Subgraph, graph: &Graph) -> Drawing {
    let width = sg.max_x - sg.min_x;
    let height = sg.max_y - sg.min_y;
    if width <= 0 || height <= 0 {
        return mk_drawing(0, 0);
    }
    let mut drawing = mk_drawing(width, height);
    if !graph.use_ascii {
        for x in 1..width {
            set_cell(&mut drawing, x, 0, "─");
            set_cell(&mut drawing, x, height, "─");
        }
        for y in 1..height {
            set_cell(&mut drawing, 0, y, "│");
            set_cell(&mut drawing, width, y, "│");
        }
        set_cell(&mut drawing, 0, 0, "┌");
        set_cell(&mut drawing, width, 0, "┐");
        set_cell(&mut drawing, 0, height, "└");
        set_cell(&mut drawing, width, height, "┘");
    } else {
        for x in 1..width {
            set_cell(&mut drawing, x, 0, "-");
            set_cell(&mut drawing, x, height, "-");
        }
        for y in 1..height {
            set_cell(&mut drawing, 0, y, "|");
            set_cell(&mut drawing, width, y, "|");
        }
        set_cell(&mut drawing, 0, 0, "+");
        set_cell(&mut drawing, width, 0, "+");
        set_cell(&mut drawing, 0, height, "+");
        set_cell(&mut drawing, width, height, "+");
    }
    drawing
}

fn draw_subgraph_label(sg: &Subgraph) -> (Drawing, DrawingCoord) {
    let width = sg.max_x - sg.min_x;
    let height = sg.max_y - sg.min_y;
    if width <= 0 || height <= 0 {
        return (mk_drawing(0, 0), DrawingCoord { x: 0, y: 0 });
    }
    let mut drawing = mk_drawing(width, height);
    let label_y = 1;
    let mut label_x = width / 2 - (sg.name.chars().count() as i32) / 2;
    if label_x < 1 {
        label_x = 1;
    }
    for (i, ch) in sg.name.chars().enumerate() {
        let x = label_x + i as i32;
        if x < width {
            set_cell(&mut drawing, x, label_y, &ch.to_string());
        }
    }
    (
        drawing,
        DrawingCoord {
            x: sg.min_x,
            y: sg.min_y,
        },
    )
}

fn wrap_text_in_color(text: String, color: Option<&String>, style_type: &str) -> String {
    let Some(color) = color else { return text };
    if style_type == "html" {
        format!("<span style='color: {}'>{}</span>", color, text)
    } else {
        text
    }
}

fn mk_drawing(x: i32, y: i32) -> Drawing {
    let mut drawing = Vec::new();
    for _ in 0..=x {
        let mut column = Vec::new();
        for _ in 0..=y {
            column.push(" ".to_string());
        }
        drawing.push(column);
    }
    drawing
}

fn get_drawing_size(drawing: &Drawing) -> (i32, i32) {
    if drawing.is_empty() {
        return (0, 0);
    }
    (drawing.len() as i32 - 1, drawing[0].len() as i32 - 1)
}

fn increase_size(drawing: &mut Drawing, x: i32, y: i32) {
    let (curr_x, curr_y) = get_drawing_size(drawing);
    let new_x = max(x, curr_x);
    let new_y = max(y, curr_y);
    let mut new_drawing = mk_drawing(new_x, new_y);
    for x in 0..=new_x {
        for y in 0..=new_y {
            if (x as usize) < drawing.len() && (y as usize) < drawing[0].len() {
                new_drawing[x as usize][y as usize] = drawing[x as usize][y as usize].clone();
            }
        }
    }
    *drawing = new_drawing;
}

fn copy_canvas(drawing: &Drawing) -> Drawing {
    let (x, y) = get_drawing_size(drawing);
    mk_drawing(x, y)
}

fn drawing_to_string(drawing: &Drawing) -> String {
    let (max_x, max_y) = get_drawing_size(drawing);
    let mut out = String::new();
    for y in 0..=max_y {
        for x in 0..=max_x {
            out.push_str(&drawing[x as usize][y as usize]);
        }
        if y != max_y {
            out.push('\n');
        }
    }
    out
}

fn set_cell(drawing: &mut Drawing, x: i32, y: i32, value: &str) {
    if x < 0 || y < 0 {
        return;
    }
    let (max_x, max_y) = get_drawing_size(drawing);
    if x > max_x || y > max_y {
        increase_size(drawing, x, y);
    }
    if let Some(cell) = drawing.get_mut(x as usize).and_then(|col| col.get_mut(y as usize)) {
        *cell = value.to_string();
    }
}

fn merge_junctions(c1: &str, c2: &str) -> String {
    let mut map = HashMap::new();
    map.insert("─", vec![("│", "┼"), ("┌", "┬"), ("┐", "┬"), ("└", "┴"), ("┘", "┴"), ("├", "┼"), ("┤", "┼"), ("┬", "┬"), ("┴", "┴")]);
    map.insert("│", vec![("─", "┼"), ("┌", "├"), ("┐", "┤"), ("└", "├"), ("┘", "┤"), ("├", "├"), ("┤", "┤"), ("┬", "┼"), ("┴", "┼")]);
    map.insert("┌", vec![("─", "┬"), ("│", "├"), ("┐", "┬"), ("└", "├"), ("┘", "┼"), ("├", "├"), ("┤", "┼"), ("┬", "┬"), ("┴", "┼")]);
    map.insert("┐", vec![("─", "┬"), ("│", "┤"), ("┌", "┬"), ("└", "┼"), ("┘", "┤"), ("├", "┼"), ("┤", "┤"), ("┬", "┬"), ("┴", "┼")]);
    map.insert("└", vec![("─", "┴"), ("│", "├"), ("┌", "├"), ("┐", "┼"), ("┘", "┴"), ("├", "├"), ("┤", "┼"), ("┬", "┼"), ("┴", "┴")]);
    map.insert("┘", vec![("─", "┴"), ("│", "┤"), ("┌", "┼"), ("┐", "┤"), ("└", "┴"), ("├", "┼"), ("┤", "┤"), ("┬", "┼"), ("┴", "┴")]);
    map.insert("├", vec![("─", "┼"), ("│", "├"), ("┌", "├"), ("┐", "┼"), ("└", "├"), ("┘", "┼"), ("┤", "┼"), ("┬", "┼"), ("┴", "┼")]);
    map.insert("┤", vec![("─", "┼"), ("│", "┤"), ("┌", "┼"), ("┐", "┤"), ("└", "┼"), ("┘", "┤"), ("├", "┼"), ("┬", "┼"), ("┴", "┼")]);
    map.insert("┬", vec![("─", "┬"), ("│", "┼"), ("┌", "┬"), ("┐", "┬"), ("└", "┼"), ("┘", "┼"), ("├", "┼"), ("┤", "┼"), ("┴", "┼")]);
    map.insert("┴", vec![("─", "┴"), ("│", "┼"), ("┌", "┼"), ("┐", "┼"), ("└", "┴"), ("┘", "┴"), ("├", "┼"), ("┤", "┼"), ("┬", "┼")]);

    if let Some(entries) = map.get(c1) {
        for (other, merged) in entries {
            if *other == c2 {
                return merged.to_string();
            }
        }
    }
    c1.to_string()
}

fn is_junction_char(c: &str) -> bool {
    matches!(
        c,
        "─" | "│" | "┌" | "┐" | "└" | "┘" | "├" | "┤" | "┬" | "┴" | "┼" | "╴" | "╵" | "╶" | "╷"
    )
}

fn merge_drawings(base: &Drawing, offset: DrawingCoord, drawings: &[Drawing], use_ascii: bool) -> Drawing {
    let (mut max_x, mut max_y) = get_drawing_size(base);
    for drawing in drawings {
        let (x, y) = get_drawing_size(drawing);
        max_x = max(max_x, x + offset.x);
        max_y = max(max_y, y + offset.y);
    }
    let mut merged = mk_drawing(max_x, max_y);
    for x in 0..=max_x {
        for y in 0..=max_y {
            if (x as usize) < base.len() && (y as usize) < base[0].len() {
                merged[x as usize][y as usize] = base[x as usize][y as usize].clone();
            }
        }
    }

    for drawing in drawings {
        for x in 0..drawing.len() {
            for y in 0..drawing[0].len() {
                let value = &drawing[x][y];
                if value != " " {
                    let target_x = (x as i32 + offset.x) as usize;
                    let target_y = (y as i32 + offset.y) as usize;
                    let current = merged[target_x][target_y].clone();
                    if !use_ascii && is_junction_char(value) && is_junction_char(&current) {
                        merged[target_x][target_y] = merge_junctions(&current, value);
                    } else {
                        merged[target_x][target_y] = value.clone();
                    }
                }
            }
        }
    }
    merged
}

impl Graph {
    fn merge_drawings(&self, base: &Drawing, offset: DrawingCoord, drawings: &[Drawing]) -> Drawing {
        merge_drawings(base, offset, drawings, self.use_ascii)
    }
}

fn draw_text_on_line(drawing: &mut Drawing, line: &[DrawingCoord], label: &str) {
    if line.len() < 2 {
        return;
    }
    let (min_x, max_x) = if line[0].x > line[1].x {
        (line[1].x, line[0].x)
    } else {
        (line[0].x, line[1].x)
    };
    let (min_y, max_y) = if line[0].y > line[1].y {
        (line[1].y, line[0].y)
    } else {
        (line[0].y, line[1].y)
    };
    let middle_x = min_x + (max_x - min_x) / 2;
    let middle_y = min_y + (max_y - min_y) / 2;
    let start_x = middle_x - (label.chars().count() as i32) / 2;
    draw_text(drawing, DrawingCoord { x: start_x, y: middle_y }, label);
}

fn draw_text(drawing: &mut Drawing, start: DrawingCoord, text: &str) {
    increase_size(drawing, start.x + text.chars().count() as i32, start.y);
    for (i, ch) in text.chars().enumerate() {
        set_cell(drawing, start.x + i as i32, start.y, &ch.to_string());
    }
}

fn debug_drawing_wrapper(drawing: &Drawing) -> Drawing {
    let (max_x, max_y) = get_drawing_size(drawing);
    let mut debug = mk_drawing(max_x + 2, max_y + 1);
    for x in 0..=max_x {
        set_cell(&mut debug, x + 2, 0, &format!("{}", x % 10));
    }
    for y in 0..=max_y {
        set_cell(&mut debug, 0, y + 1, &format!("{:2}", y));
    }
    for x in 0..debug.len() {
        for y in 0..debug[0].len() {
            let src_x = x as i32 - 2;
            let src_y = y as i32 - 1;
            if src_x >= 0 && src_y >= 0 {
                if (src_x as usize) < drawing.len() && (src_y as usize) < drawing[0].len() {
                    debug[x][y] = drawing[src_x as usize][src_y as usize].clone();
                }
            }
        }
    }
    debug
}

fn debug_coord_wrapper(drawing: &Drawing, graph: &Graph) -> Drawing {
    let (max_x, max_y) = get_drawing_size(drawing);
    let mut debug = mk_drawing(max_x + 2, max_y + 1);
    let mut curr_x = 3;
    for x in 0..100 {
        let w = graph.column_width.get(&x).copied().unwrap_or(0);
        if curr_x > max_x + w {
            break;
        }
        set_cell(&mut debug, curr_x, 0, &format!("{}", x % 10));
        curr_x += w;
    }
    let mut curr_y = 2;
    for y in 0..100 {
        let h = graph.row_height.get(&y).copied().unwrap_or(0);
        if curr_y > max_y + h {
            break;
        }
        let pos = curr_y + h / 2;
        set_cell(&mut debug, 0, pos, &format!("{}", y % 10));
        curr_y += h;
    }

    merge_drawings(&debug, DrawingCoord { x: 1, y: 1 }, &[drawing.clone()], graph.use_ascii)
}

fn min(x: i32, y: i32) -> i32 {
    if x < y {
        x
    } else {
        y
    }
}

fn max(x: i32, y: i32) -> i32 {
    if x > y {
        x
    } else {
        y
    }
}

fn ceil_div(x: i32, y: i32) -> i32 {
    if x % y == 0 {
        x / y
    } else {
        x / y + 1
    }
}
