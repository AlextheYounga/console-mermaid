use crate::graph::draw::{draw_box, increase_size, mk_drawing};
use crate::graph::types::{
    DrawingCoord, Graph, GraphProperties, GridCoord, MIDDLE, QueueItem, Subgraph,
    determine_start_and_end_dir, heuristic, max, merge_path, min,
};
use std::collections::{BinaryHeap, HashMap, HashSet};

pub(crate) fn mk_graph(properties: &GraphProperties) -> Graph {
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
        let parent_label = properties
            .node_labels
            .get(node_name)
            .cloned()
            .unwrap_or_else(|| node_name.clone());
        let (parent_idx, _) = graph.get_or_insert_node(node_name, &parent_label, "");
        for edge in children {
            let child_label = properties
                .node_labels
                .get(&edge.child.name)
                .cloned()
                .unwrap_or_else(|| edge.child.label.clone());
            let (child_idx, inserted) =
                graph.get_or_insert_node(&edge.child.name, &child_label, &edge.get_child_style());
            if inserted {
                graph.nodes[parent_idx].style_class_name = edge.parent.style_class.clone();
            }
            graph.edges.push(crate::graph::types::Edge {
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

impl Graph {
    pub(crate) fn get_or_insert_node(
        &mut self,
        name: &str,
        label: &str,
        style_class: &str,
    ) -> (usize, bool) {
        if let Some(idx) = self.node_index_by_name.get(name) {
            if let Some(node) = self.nodes.get_mut(*idx) {
                if label != name {
                    node.label = label.to_string();
                }
            }
            return (*idx, false);
        }
        let idx = self.nodes.len();
        self.nodes.push(crate::graph::types::Node {
            name: name.to_string(),
            label: label.to_string(),
            drawing: None,
            drawing_coord: None,
            grid_coord: None,
            drawn: false,
            index: idx,
            style_class_name: style_class.to_string(),
            style_class: crate::graph::types::StyleClass::default(),
        });
        self.node_index_by_name.insert(name.to_string(), idx);
        (idx, true)
    }

    pub(crate) fn set_style_classes(&mut self, properties: &GraphProperties) {
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

    pub(crate) fn set_subgraphs(&mut self, text_subgraphs: &[crate::graph::types::TextSubgraph]) {
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

    pub(crate) fn create_mapping(&mut self) {
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

        let should_separate =
            self.graph_direction == "LR" && has_external_roots && has_subgraph_roots_with_edges;
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
                self.reserve_spot_in_grid(
                    *idx,
                    GridCoord {
                        x: 0,
                        y: highest_position_per_level[0],
                    },
                )
            } else {
                self.reserve_spot_in_grid(
                    *idx,
                    GridCoord {
                        x: highest_position_per_level[0],
                        y: 0,
                    },
                )
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

    pub(crate) fn set_column_width(&mut self, idx: usize) {
        let node = &self.nodes[idx];
        let grid_coord = node.grid_coord.unwrap();
        let name_len = node.label.chars().count() as i32;
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

    pub(crate) fn increase_grid_size_for_path(&mut self, path: &[GridCoord]) {
        for coord in path {
            self.column_width
                .entry(coord.x)
                .or_insert(self.padding_x / 2);
            self.row_height.entry(coord.y).or_insert(self.padding_y / 2);
        }
    }

    pub(crate) fn reserve_spot_in_grid(
        &mut self,
        node_idx: usize,
        requested: GridCoord,
    ) -> GridCoord {
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

    pub(crate) fn get_children(&self, node_idx: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|edge| edge.from == node_idx)
            .map(|edge| edge.to)
            .collect()
    }

    pub(crate) fn grid_to_drawing_coord(
        &self,
        coord: GridCoord,
        dir: Option<crate::graph::types::Direction>,
    ) -> DrawingCoord {
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

    pub(crate) fn determine_path(&mut self, edge_idx: usize) {
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

    pub(crate) fn determine_label_line(&mut self, edge_idx: usize) {
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

    pub(crate) fn calculate_line_width(&self, line: &[GridCoord]) -> i32 {
        line.iter()
            .map(|c| *self.column_width.get(&c.x).unwrap_or(&0))
            .sum()
    }

    pub(crate) fn calculate_subgraph_bounding_boxes(&mut self) {
        for idx in 0..self.subgraphs.len() {
            self.calculate_subgraph_bounding_box(idx);
        }
        self.ensure_subgraph_spacing();
    }

    pub(crate) fn calculate_subgraph_bounding_box(&mut self, idx: usize) {
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

    pub(crate) fn ensure_subgraph_spacing(&mut self) {
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

    pub(crate) fn offset_drawing_for_subgraphs(&mut self) {
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

    pub(crate) fn is_node_in_any_subgraph(&self, node_idx: usize) -> bool {
        self.subgraphs
            .iter()
            .any(|sg| sg.nodes.iter().any(|idx| *idx == node_idx))
    }

    pub(crate) fn get_node_subgraph(&self, node_idx: usize) -> Option<usize> {
        self.subgraphs
            .iter()
            .position(|sg| sg.nodes.iter().any(|idx| *idx == node_idx))
    }

    pub(crate) fn has_incoming_edge_from_outside_subgraph(&self, node_idx: usize) -> bool {
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
                if let (Some(other_coord), Some(node_coord)) =
                    (other_coord, self.nodes[node_idx].grid_coord)
                {
                    if other_coord.y < node_coord.y {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub(crate) fn set_drawing_size_to_grid_constraints(&mut self) {
        let max_x: i32 = self.column_width.values().sum();
        let max_y: i32 = self.row_height.values().sum();
        let drawing = &mut self.drawing;
        increase_size(drawing, max_x - 1, max_y - 1);
    }

    pub(crate) fn get_path(
        &self,
        from: GridCoord,
        to: GridCoord,
    ) -> Result<Vec<GridCoord>, String> {
        let mut pq = BinaryHeap::new();
        pq.push(QueueItem {
            coord: from,
            priority: 0,
        });
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
                    pq.push(QueueItem {
                        coord: next,
                        priority,
                    });
                    came_from.insert(next, Some(current));
                }
            }
        }

        Err("no path found".to_string())
    }

    pub(crate) fn is_free_in_grid(&self, coord: GridCoord) -> bool {
        if coord.x < 0 || coord.y < 0 {
            return false;
        }
        !self.grid.contains_key(&coord)
    }
}
