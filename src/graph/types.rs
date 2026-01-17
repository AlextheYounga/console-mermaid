use indexmap::IndexMap;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub(crate) struct TextNode {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) style_class: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TextEdge {
    pub(crate) parent: TextNode,
    pub(crate) child: TextNode,
    pub(crate) label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TextSubgraph {
    pub(crate) name: String,
    pub(crate) nodes: Vec<String>,
    pub(crate) parent: Option<usize>,
    pub(crate) children: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct GraphProperties {
    pub(crate) data: IndexMap<String, Vec<TextEdge>>,
    pub(crate) style_classes: HashMap<String, StyleClass>,
    pub(crate) node_labels: HashMap<String, String>,
    pub(crate) graph_direction: String,
    pub(crate) style_type: String,
    pub(crate) padding_x: i32,
    pub(crate) padding_y: i32,
    pub(crate) box_border_padding: i32,
    pub(crate) subgraphs: Vec<TextSubgraph>,
    pub(crate) use_ascii: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StyleClass {
    pub(crate) name: String,
    pub(crate) styles: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GenericCoord {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, Eq)]
pub(crate) struct GridCoord {
    pub(crate) x: i32,
    pub(crate) y: i32,
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
pub(crate) struct DrawingCoord {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl PartialEq for DrawingCoord {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl GridCoord {
    pub(crate) fn equals(&self, other: GridCoord) -> bool {
        self.x == other.x && self.y == other.y
    }

    pub(crate) fn direction(&self, dir: Direction) -> GridCoord {
        GridCoord {
            x: self.x + dir.dx,
            y: self.y + dir.dy,
        }
    }
}

impl DrawingCoord {
    pub(crate) fn equals(&self, other: DrawingCoord) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Direction {
    pub(crate) dx: i32,
    pub(crate) dy: i32,
}

pub(crate) const UP: Direction = Direction { dx: 1, dy: 0 };
pub(crate) const DOWN: Direction = Direction { dx: 1, dy: 2 };
pub(crate) const LEFT: Direction = Direction { dx: 0, dy: 1 };
pub(crate) const RIGHT: Direction = Direction { dx: 2, dy: 1 };
pub(crate) const UPPER_RIGHT: Direction = Direction { dx: 2, dy: 0 };
pub(crate) const UPPER_LEFT: Direction = Direction { dx: 0, dy: 0 };
pub(crate) const LOWER_RIGHT: Direction = Direction { dx: 2, dy: 2 };
pub(crate) const LOWER_LEFT: Direction = Direction { dx: 0, dy: 2 };
pub(crate) const MIDDLE: Direction = Direction { dx: 1, dy: 1 };

impl Direction {
    pub(crate) fn opposite(self) -> Direction {
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

pub(crate) type Drawing = Vec<Vec<String>>;

#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) drawing: Option<Drawing>,
    pub(crate) drawing_coord: Option<DrawingCoord>,
    pub(crate) grid_coord: Option<GridCoord>,
    pub(crate) drawn: bool,
    pub(crate) index: usize,
    pub(crate) style_class_name: String,
    pub(crate) style_class: StyleClass,
}

#[derive(Debug, Clone)]
pub(crate) struct Edge {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) text: String,
    pub(crate) path: Vec<GridCoord>,
    pub(crate) label_line: Vec<GridCoord>,
    pub(crate) start_dir: Direction,
    pub(crate) end_dir: Direction,
}

#[derive(Debug, Clone)]
pub(crate) struct Subgraph {
    pub(crate) name: String,
    pub(crate) nodes: Vec<usize>,
    pub(crate) parent: Option<usize>,
    pub(crate) children: Vec<usize>,
    pub(crate) min_x: i32,
    pub(crate) min_y: i32,
    pub(crate) max_x: i32,
    pub(crate) max_y: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct Graph {
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) drawing: Drawing,
    pub(crate) grid: HashMap<GridCoord, usize>,
    pub(crate) column_width: HashMap<i32, i32>,
    pub(crate) row_height: HashMap<i32, i32>,
    pub(crate) style_classes: HashMap<String, StyleClass>,
    pub(crate) style_type: String,
    pub(crate) padding_x: i32,
    pub(crate) padding_y: i32,
    pub(crate) box_border_padding: i32,
    pub(crate) subgraphs: Vec<Subgraph>,
    pub(crate) offset_x: i32,
    pub(crate) offset_y: i32,
    pub(crate) use_ascii: bool,
    pub(crate) graph_direction: String,
    pub(crate) node_index_by_name: HashMap<String, usize>,
}

impl TextEdge {
    pub(crate) fn get_child_style(&self) -> String {
        self.child.style_class.clone()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct QueueItem {
    pub(crate) coord: GridCoord,
    pub(crate) priority: i32,
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

pub(crate) fn heuristic(a: GridCoord, b: GridCoord) -> i32 {
    let abs_x = (a.x - b.x).abs();
    let abs_y = (a.y - b.y).abs();
    if abs_x == 0 || abs_y == 0 {
        abs_x + abs_y
    } else {
        abs_x + abs_y + 1
    }
}

pub(crate) fn merge_path(path: Vec<GridCoord>) -> Vec<GridCoord> {
    if path.len() <= 2 {
        return path;
    }
    let mut remove: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut step0 = path[0];
    let mut step1 = path[1];
    for (idx, step2) in path.iter().skip(2).enumerate() {
        let prev_dir = determine_direction(
            GenericCoord {
                x: step0.x,
                y: step0.y,
            },
            GenericCoord {
                x: step1.x,
                y: step1.y,
            },
        );
        let dir = determine_direction(
            GenericCoord {
                x: step1.x,
                y: step1.y,
            },
            GenericCoord {
                x: step2.x,
                y: step2.y,
            },
        );
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

pub(crate) fn determine_direction(from: GenericCoord, to: GenericCoord) -> Direction {
    if from.x == to.x {
        if from.y < to.y { DOWN } else { UP }
    } else if from.y == to.y {
        if from.x < to.x { RIGHT } else { LEFT }
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

pub(crate) fn self_reference_direction(
    graph_direction: &str,
) -> (Direction, Direction, Direction, Direction) {
    if graph_direction == "LR" {
        (RIGHT, DOWN, DOWN, RIGHT)
    } else {
        (DOWN, RIGHT, RIGHT, DOWN)
    }
}

pub(crate) fn determine_start_and_end_dir(
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
        GenericCoord {
            x: from_coord.x,
            y: from_coord.y,
        },
        GenericCoord {
            x: to_coord.x,
            y: to_coord.y,
        },
    );
    let is_backwards = if graph_direction == "LR" {
        d == LEFT || d == UPPER_LEFT || d == LOWER_LEFT
    } else {
        d == UP || d == UPPER_LEFT || d == UPPER_RIGHT
    };

    let (mut preferred_dir, mut preferred_opp, mut alt_dir, mut alt_opp) =
        (d, d.opposite(), d, d.opposite());
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

pub(crate) fn min(x: i32, y: i32) -> i32 {
    if x < y { x } else { y }
}

pub(crate) fn max(x: i32, y: i32) -> i32 {
    if x > y { x } else { y }
}

pub(crate) fn ceil_div(x: i32, y: i32) -> i32 {
    if x % y == 0 { x / y } else { x / y + 1 }
}
