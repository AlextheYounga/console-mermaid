use crate::graph::layout::mk_graph;
use crate::graph::types::{
    DOWN, Direction, Drawing, DrawingCoord, Edge, GenericCoord, Graph, GraphProperties, GridCoord,
    LEFT, LOWER_LEFT, LOWER_RIGHT, Node, RIGHT, Subgraph, UP, UPPER_LEFT, UPPER_RIGHT, ceil_div,
    determine_direction, max,
};
use std::collections::HashMap;

pub(crate) fn draw_map(properties: &GraphProperties, show_coords: bool) -> Result<String, String> {
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

impl Graph {
    pub(crate) fn draw(&mut self) -> Drawing {
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

        self.drawing =
            self.merge_drawings(&self.drawing, DrawingCoord { x: 0, y: 0 }, &line_drawings);
        self.drawing =
            self.merge_drawings(&self.drawing, DrawingCoord { x: 0, y: 0 }, &corner_drawings);
        self.drawing = self.merge_drawings(
            &self.drawing,
            DrawingCoord { x: 0, y: 0 },
            &arrow_head_drawings,
        );
        self.drawing = self.merge_drawings(
            &self.drawing,
            DrawingCoord { x: 0, y: 0 },
            &box_start_drawings,
        );
        self.drawing =
            self.merge_drawings(&self.drawing, DrawingCoord { x: 0, y: 0 }, &label_drawings);

        self.draw_subgraph_labels();

        self.drawing.clone()
    }

    pub(crate) fn draw_node(&mut self, idx: usize) {
        if let Some(coord) = self.nodes[idx].drawing_coord {
            if let Some(drawing) = &self.nodes[idx].drawing {
                self.drawing = self.merge_drawings(&self.drawing, coord, &[drawing.clone()]);
                self.nodes[idx].drawn = true;
            }
        }
    }

    pub(crate) fn draw_edge(
        &self,
        edge_idx: usize,
    ) -> (Drawing, Drawing, Drawing, Drawing, Drawing) {
        let edge = &self.edges[edge_idx];
        if edge.path.is_empty() {
            return (
                mk_drawing(0, 0),
                mk_drawing(0, 0),
                mk_drawing(0, 0),
                mk_drawing(0, 0),
                mk_drawing(0, 0),
            );
        }
        let from = self.nodes[edge.from]
            .grid_coord
            .unwrap()
            .direction(edge.start_dir);
        let to = self.nodes[edge.to]
            .grid_coord
            .unwrap()
            .direction(edge.end_dir);
        self.draw_arrow(from, to, edge)
    }

    pub(crate) fn draw_subgraphs(&mut self) {
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

    pub(crate) fn draw_subgraph_labels(&mut self) {
        for sg in &self.subgraphs {
            if sg.nodes.is_empty() {
                continue;
            }
            let (label, offset) = draw_subgraph_label(sg);
            self.drawing = self.merge_drawings(&self.drawing, offset, &[label]);
        }
    }

    pub(crate) fn sort_subgraphs_by_depth(&self) -> Vec<usize> {
        let mut depths = HashMap::new();
        for (idx, _) in self.subgraphs.iter().enumerate() {
            let depth = self.get_subgraph_depth(idx);
            depths.insert(idx, depth);
        }
        let mut sorted: Vec<usize> = (0..self.subgraphs.len()).collect();
        sorted.sort_by_key(|idx| depths.get(idx).copied().unwrap_or(0));
        sorted
    }

    pub(crate) fn get_subgraph_depth(&self, idx: usize) -> i32 {
        if let Some(parent) = self.subgraphs[idx].parent {
            1 + self.get_subgraph_depth(parent)
        } else {
            0
        }
    }

    pub(crate) fn draw_arrow(
        &self,
        _from: GridCoord,
        _to: GridCoord,
        edge: &Edge,
    ) -> (Drawing, Drawing, Drawing, Drawing, Drawing) {
        if edge.path.is_empty() {
            return (
                mk_drawing(0, 0),
                mk_drawing(0, 0),
                mk_drawing(0, 0),
                mk_drawing(0, 0),
                mk_drawing(0, 0),
            );
        }
        let label = self.draw_arrow_label(edge);
        let (path, lines_drawn, _line_dirs) = self.draw_path(&edge.path);
        let box_start = self.draw_box_start(&edge.path, &lines_drawn[0]);
        let arrow_head = self.draw_arrow_head(lines_drawn.last().unwrap(), edge.end_dir.opposite());
        let corners = self.draw_corners(&edge.path);
        (path, box_start, arrow_head, corners, label)
    }

    pub(crate) fn draw_path(
        &self,
        path: &[GridCoord],
    ) -> (Drawing, Vec<Vec<DrawingCoord>>, Vec<Direction>) {
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
            let dir = determine_direction(
                GenericCoord {
                    x: previous.x,
                    y: previous.y,
                },
                GenericCoord {
                    x: next.x,
                    y: next.y,
                },
            );
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

    pub(crate) fn draw_line(
        &self,
        drawing: &mut Drawing,
        from: DrawingCoord,
        to: DrawingCoord,
        offset_from: i32,
        offset_to: i32,
    ) -> Vec<DrawingCoord> {
        let dir = determine_direction(
            GenericCoord {
                x: from.x,
                y: from.y,
            },
            GenericCoord { x: to.x, y: to.y },
        );
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

    pub(crate) fn draw_box_start(
        &self,
        path: &[GridCoord],
        first_line: &[DrawingCoord],
    ) -> Drawing {
        let mut drawing = copy_canvas(&self.drawing);
        if self.use_ascii || first_line.is_empty() {
            return drawing;
        }
        let dir = determine_direction(
            GenericCoord {
                x: path[0].x,
                y: path[0].y,
            },
            GenericCoord {
                x: path[1].x,
                y: path[1].y,
            },
        );
        let from = if dir == UP || dir == LEFT {
            first_line[first_line.len() - 1]
        } else {
            first_line[0]
        };
        match dir {
            d if d == UP => set_cell(&mut drawing, from.x, from.y + 1, "┴"),
            d if d == DOWN => set_cell(&mut drawing, from.x, from.y - 1, "┬"),
            d if d == LEFT => set_cell(&mut drawing, from.x + 1, from.y, "┤"),
            d if d == RIGHT => set_cell(&mut drawing, from.x - 1, from.y, "├"),
            _ => {}
        }
        drawing
    }

    pub(crate) fn draw_arrow_head(&self, line: &[DrawingCoord], arrow_dir: Direction) -> Drawing {
        let mut drawing = copy_canvas(&self.drawing);
        if line.is_empty() {
            return drawing;
        }
        let dir = arrow_dir;
        let head = if dir == UP || dir == LEFT {
            line[0]
        } else {
            line[line.len() - 1]
        };

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
                _ => match arrow_dir {
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
                _ => match arrow_dir {
                    d if d == UP => "^",
                    d if d == DOWN => "v",
                    d if d == LEFT => "<",
                    d if d == RIGHT => ">",
                    _ => "*",
                },
            }
        };

        set_cell(&mut drawing, head.x, head.y, ch);
        drawing
    }

    pub(crate) fn draw_corners(&self, path: &[GridCoord]) -> Drawing {
        let mut drawing = copy_canvas(&self.drawing);
        for idx in 1..path.len().saturating_sub(1) {
            let coord = path[idx];
            let drawing_coord = self.grid_to_drawing_coord(coord, None);
            let prev_dir = determine_direction(
                GenericCoord {
                    x: path[idx - 1].x,
                    y: path[idx - 1].y,
                },
                GenericCoord {
                    x: coord.x,
                    y: coord.y,
                },
            );
            let next_dir = determine_direction(
                GenericCoord {
                    x: coord.x,
                    y: coord.y,
                },
                GenericCoord {
                    x: path[idx + 1].x,
                    y: path[idx + 1].y,
                },
            );
            let corner = if !self.use_ascii {
                if (prev_dir == RIGHT && next_dir == DOWN) || (prev_dir == UP && next_dir == LEFT) {
                    "┐"
                } else if (prev_dir == RIGHT && next_dir == UP)
                    || (prev_dir == DOWN && next_dir == LEFT)
                {
                    "┘"
                } else if (prev_dir == LEFT && next_dir == DOWN)
                    || (prev_dir == UP && next_dir == RIGHT)
                {
                    "┌"
                } else if (prev_dir == LEFT && next_dir == UP)
                    || (prev_dir == DOWN && next_dir == RIGHT)
                {
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

    pub(crate) fn draw_arrow_label(&self, edge: &Edge) -> Drawing {
        let mut drawing = copy_canvas(&self.drawing);
        if edge.text.is_empty() || edge.label_line.len() < 2 {
            return drawing;
        }
        let line = self.line_to_drawing(&edge.label_line);
        draw_text_on_line(&mut drawing, &line, &edge.text);
        drawing
    }

    pub(crate) fn line_to_drawing(&self, line: &[GridCoord]) -> Vec<DrawingCoord> {
        line.iter()
            .map(|coord| self.grid_to_drawing_coord(*coord, None))
            .collect()
    }
}

pub(crate) fn draw_box(node: &Node, graph: &Graph) -> Drawing {
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
    let name_len = node.label.chars().count() as i32;
    let text_x = w / 2 - ceil_div(name_len, 2) + 1;
    for (i, ch) in node.label.chars().enumerate() {
        let wrapped = wrap_text_in_color(
            ch.to_string(),
            node.style_class.styles.get("color"),
            &graph.style_type,
        );
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

pub(crate) fn mk_drawing(x: i32, y: i32) -> Drawing {
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

pub(crate) fn increase_size(drawing: &mut Drawing, x: i32, y: i32) {
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
    if let Some(cell) = drawing
        .get_mut(x as usize)
        .and_then(|col| col.get_mut(y as usize))
    {
        *cell = value.to_string();
    }
}

fn merge_junctions(c1: &str, c2: &str) -> String {
    let mut map = HashMap::new();
    map.insert(
        "─",
        vec![
            ("│", "┼"),
            ("┌", "┬"),
            ("┐", "┬"),
            ("└", "┴"),
            ("┘", "┴"),
            ("├", "┼"),
            ("┤", "┼"),
            ("┬", "┬"),
            ("┴", "┴"),
        ],
    );
    map.insert(
        "│",
        vec![
            ("─", "┼"),
            ("┌", "├"),
            ("┐", "┤"),
            ("└", "├"),
            ("┘", "┤"),
            ("├", "├"),
            ("┤", "┤"),
            ("┬", "┼"),
            ("┴", "┼"),
        ],
    );
    map.insert(
        "┌",
        vec![
            ("─", "┬"),
            ("│", "├"),
            ("┐", "┬"),
            ("└", "├"),
            ("┘", "┼"),
            ("├", "├"),
            ("┤", "┼"),
            ("┬", "┬"),
            ("┴", "┼"),
        ],
    );
    map.insert(
        "┐",
        vec![
            ("─", "┬"),
            ("│", "┤"),
            ("┌", "┬"),
            ("└", "┼"),
            ("┘", "┤"),
            ("├", "┼"),
            ("┤", "┤"),
            ("┬", "┬"),
            ("┴", "┼"),
        ],
    );
    map.insert(
        "└",
        vec![
            ("─", "┴"),
            ("│", "├"),
            ("┌", "├"),
            ("┐", "┼"),
            ("┘", "┴"),
            ("├", "├"),
            ("┤", "┼"),
            ("┬", "┼"),
            ("┴", "┴"),
        ],
    );
    map.insert(
        "┘",
        vec![
            ("─", "┴"),
            ("│", "┤"),
            ("┌", "┼"),
            ("┐", "┤"),
            ("└", "┴"),
            ("├", "┼"),
            ("┤", "┤"),
            ("┬", "┼"),
            ("┴", "┴"),
        ],
    );
    map.insert(
        "├",
        vec![
            ("─", "┼"),
            ("│", "├"),
            ("┌", "├"),
            ("┐", "┼"),
            ("└", "├"),
            ("┘", "┼"),
            ("┤", "┼"),
            ("┬", "┼"),
            ("┴", "┼"),
        ],
    );
    map.insert(
        "┤",
        vec![
            ("─", "┼"),
            ("│", "┤"),
            ("┌", "┼"),
            ("┐", "┤"),
            ("└", "┼"),
            ("┘", "┤"),
            ("├", "┼"),
            ("┬", "┼"),
            ("┴", "┼"),
        ],
    );
    map.insert(
        "┬",
        vec![
            ("─", "┬"),
            ("│", "┼"),
            ("┌", "┬"),
            ("┐", "┬"),
            ("└", "┼"),
            ("┘", "┼"),
            ("├", "┼"),
            ("┤", "┼"),
            ("┴", "┼"),
        ],
    );
    map.insert(
        "┴",
        vec![
            ("─", "┴"),
            ("│", "┼"),
            ("┌", "┼"),
            ("┐", "┼"),
            ("└", "┴"),
            ("┘", "┴"),
            ("├", "┼"),
            ("┤", "┼"),
            ("┬", "┼"),
        ],
    );

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
        "─" | "│"
            | "┌"
            | "┐"
            | "└"
            | "┘"
            | "├"
            | "┤"
            | "┬"
            | "┴"
            | "┼"
            | "╴"
            | "╵"
            | "╶"
            | "╷"
    )
}

fn junction_dirs(c: &str) -> (bool, bool, bool, bool) {
    match c {
        "─" => (false, false, true, true),
        "│" => (true, true, false, false),
        "┌" => (false, true, false, true),
        "┐" => (false, true, true, false),
        "└" => (true, false, false, true),
        "┘" => (true, false, true, false),
        "├" => (true, true, false, true),
        "┤" => (true, true, true, false),
        "┬" => (false, true, true, true),
        "┴" => (true, false, true, true),
        "┼" => (true, true, true, true),
        "╴" => (false, false, true, false),
        "╵" => (true, false, false, false),
        "╶" => (false, false, false, true),
        "╷" => (false, true, false, false),
        _ => (false, false, false, false),
    }
}

fn junction_from_dirs(up: bool, down: bool, left: bool, right: bool) -> &'static str {
    match (up, down, left, right) {
        (true, true, true, true) => "┼",
        (true, true, true, false) => "┤",
        (true, true, false, true) => "├",
        (true, false, true, true) => "┴",
        (false, true, true, true) => "┬",
        (false, true, false, true) => "┌",
        (false, true, true, false) => "┐",
        (true, false, false, true) => "└",
        (true, false, true, false) => "┘",
        (true, true, false, false) => "│",
        (false, false, true, true) => "─",
        (true, false, false, false) => "│",
        (false, true, false, false) => "│",
        (false, false, true, false) => "─",
        (false, false, false, true) => "─",
        _ => " ",
    }
}

fn get_cell(drawing: &Drawing, x: i32, y: i32) -> Option<&str> {
    if x < 0 || y < 0 {
        return None;
    }
    drawing
        .get(x as usize)
        .and_then(|col| col.get(y as usize))
        .map(|s| s.as_str())
}

fn merge_drawings(
    base: &Drawing,
    offset: DrawingCoord,
    drawings: &[Drawing],
    use_ascii: bool,
) -> Drawing {
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
                        let merged_value = merge_junctions(&current, value);
                        if merged_value == "┼" {
                            let (mut up, mut down, mut left, mut right) = junction_dirs(&current);
                            let (v_up, v_down, v_left, v_right) = junction_dirs(value);
                            up |= v_up;
                            down |= v_down;
                            left |= v_left;
                            right |= v_right;

                            let target_x_i32 = target_x as i32;
                            let target_y_i32 = target_y as i32;
                            let local_x = x as i32;
                            let local_y = y as i32;

                            if up {
                                let neighbor_up = get_cell(&merged, target_x_i32, target_y_i32 - 1)
                                    .map(|c| junction_dirs(c).1)
                                    .unwrap_or(false)
                                    || get_cell(drawing, local_x, local_y - 1)
                                        .map(|c| junction_dirs(c).1)
                                        .unwrap_or(false);
                                up = neighbor_up;
                            }
                            if down {
                                let neighbor_down =
                                    get_cell(&merged, target_x_i32, target_y_i32 + 1)
                                        .map(|c| junction_dirs(c).0)
                                        .unwrap_or(false)
                                        || get_cell(drawing, local_x, local_y + 1)
                                            .map(|c| junction_dirs(c).0)
                                            .unwrap_or(false);
                                down = neighbor_down;
                            }
                            if left {
                                let neighbor_left =
                                    get_cell(&merged, target_x_i32 - 1, target_y_i32)
                                        .map(|c| junction_dirs(c).3)
                                        .unwrap_or(false)
                                        || get_cell(drawing, local_x - 1, local_y)
                                            .map(|c| junction_dirs(c).3)
                                            .unwrap_or(false);
                                left = neighbor_left;
                            }
                            if right {
                                let neighbor_right =
                                    get_cell(&merged, target_x_i32 + 1, target_y_i32)
                                        .map(|c| junction_dirs(c).2)
                                        .unwrap_or(false)
                                        || get_cell(drawing, local_x + 1, local_y)
                                            .map(|c| junction_dirs(c).2)
                                            .unwrap_or(false);
                                right = neighbor_right;
                            }

                            merged[target_x][target_y] =
                                junction_from_dirs(up, down, left, right).to_string();
                        } else {
                            merged[target_x][target_y] = merged_value;
                        }
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
    pub(crate) fn merge_drawings(
        &self,
        base: &Drawing,
        offset: DrawingCoord,
        drawings: &[Drawing],
    ) -> Drawing {
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
    draw_text(
        drawing,
        DrawingCoord {
            x: start_x,
            y: middle_y,
        },
        label,
    );
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

    merge_drawings(
        &debug,
        DrawingCoord { x: 1, y: 1 },
        &[drawing.clone()],
        graph.use_ascii,
    )
}
