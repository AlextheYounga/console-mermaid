use crate::diagram::{Config, Diagram, remove_comments, split_lines};
use regex::Regex;
use unicode_width::UnicodeWidthStr;

const SEQUENCE_DIAGRAM_KEYWORD: &str = "sequenceDiagram";
const SOLID_ARROW_SYNTAX: &str = "->>";
const DOTTED_ARROW_SYNTAX: &str = "-->>";

#[derive(Debug, Clone, Copy)]
pub enum ArrowType {
    Solid,
    Dotted,
}

impl std::fmt::Display for ArrowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArrowType::Solid => write!(f, "solid"),
            ArrowType::Dotted => write!(f, "dotted"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub arrow_type: ArrowType,
    pub number: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    pub autonumber: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct BoxChars {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
    pub tee_down: char,
    pub tee_right: char,
    pub tee_left: char,
    pub cross: char,
    pub arrow_right: char,
    pub arrow_left: char,
    pub solid_line: char,
    pub dotted_line: char,
    pub self_top_right: char,
    pub self_bottom: char,
}

pub const ASCII: BoxChars = BoxChars {
    top_left: '+',
    top_right: '+',
    bottom_left: '+',
    bottom_right: '+',
    horizontal: '-',
    vertical: '|',
    tee_down: '+',
    tee_right: '+',
    tee_left: '+',
    cross: '+',
    arrow_right: '>',
    arrow_left: '<',
    solid_line: '-',
    dotted_line: '.',
    self_top_right: '+',
    self_bottom: '+',
};

pub const UNICODE: BoxChars = BoxChars {
    top_left: '┌',
    top_right: '┐',
    bottom_left: '└',
    bottom_right: '┘',
    horizontal: '─',
    vertical: '│',
    tee_down: '┬',
    tee_right: '├',
    tee_left: '┤',
    cross: '┼',
    arrow_right: '►',
    arrow_left: '◄',
    solid_line: '─',
    dotted_line: '┈',
    self_top_right: '┐',
    self_bottom: '┘',
};

pub fn is_sequence_diagram(input: &str) -> bool {
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        return trimmed.starts_with(SEQUENCE_DIAGRAM_KEYWORD);
    }
    false
}

pub fn parse(input: &str) -> Result<SequenceDiagram, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("empty input".to_string());
    }

    let raw_lines = split_lines(input);
    let lines = remove_comments(&raw_lines);
    if lines.is_empty() {
        return Err("no content found".to_string());
    }

    if !lines[0].trim().starts_with(SEQUENCE_DIAGRAM_KEYWORD) {
        return Err(format!("expected \"{}\" keyword", SEQUENCE_DIAGRAM_KEYWORD));
    }

    let participant_re =
        Regex::new(r#"^\s*participant\s+(?:"([^"]+)"|(\S+))(?:\s+as\s+(.+))?$"#).unwrap();
    let message_re = Regex::new(
        r#"^\s*(?:"([^"]+)"|([^\s\->]+))\s*(-->>|->>)\s*(?:"([^"]+)"|([^\s\->]+))\s*:\s*(.*)$"#,
    )
    .unwrap();
    let autonumber_re = Regex::new(r"^\s*autonumber\s*$").unwrap();

    let mut diagram = SequenceDiagram::default();
    let mut participants = std::collections::HashMap::new();

    for (idx, line) in lines.iter().skip(1).enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if autonumber_re.is_match(trimmed) {
            diagram.autonumber = true;
            continue;
        }

        if let Some(caps) = participant_re.captures(trimmed) {
            let id = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let id = if let Some(quoted) = caps.get(1) {
                quoted.as_str()
            } else {
                id
            };
            let label = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let label = if label.is_empty() { id } else { label };
            if participants.contains_key(id) {
                return Err(format!(
                    "line {}: duplicate participant \"{}\"",
                    idx + 2,
                    id
                ));
            }
            let participant = Participant {
                id: id.to_string(),
                label: label.trim_matches('"').to_string(),
                index: diagram.participants.len(),
            };
            participants.insert(id.to_string(), participant.index);
            diagram.participants.push(participant);
            continue;
        }

        if let Some(caps) = message_re.captures(trimmed) {
            let from_id = if let Some(quoted) = caps.get(1) {
                quoted.as_str()
            } else {
                caps.get(2).map(|m| m.as_str()).unwrap_or("")
            };
            let arrow = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let to_id = if let Some(quoted) = caps.get(4) {
                quoted.as_str()
            } else {
                caps.get(5).map(|m| m.as_str()).unwrap_or("")
            };
            let label = caps.get(6).map(|m| m.as_str()).unwrap_or("").trim();

            let from_idx = get_or_insert_participant(from_id, &mut diagram, &mut participants);
            let to_idx = get_or_insert_participant(to_id, &mut diagram, &mut participants);

            let arrow_type = if arrow == SOLID_ARROW_SYNTAX {
                ArrowType::Solid
            } else {
                ArrowType::Dotted
            };

            let number = if diagram.autonumber {
                diagram.messages.len() + 1
            } else {
                0
            };

            diagram.messages.push(Message {
                from: from_idx,
                to: to_idx,
                label: label.to_string(),
                arrow_type,
                number,
            });
            continue;
        }

        return Err(format!("line {}: invalid syntax: \"{}\"", idx + 2, trimmed));
    }

    if diagram.participants.is_empty() {
        return Err("no participants found".to_string());
    }

    Ok(diagram)
}

fn get_or_insert_participant(
    id: &str,
    diagram: &mut SequenceDiagram,
    participants: &mut std::collections::HashMap<String, usize>,
) -> usize {
    if let Some(idx) = participants.get(id) {
        return *idx;
    }
    let idx = diagram.participants.len();
    diagram.participants.push(Participant {
        id: id.to_string(),
        label: id.to_string(),
        index: idx,
    });
    participants.insert(id.to_string(), idx);
    idx
}

const DEFAULT_SELF_MESSAGE_WIDTH: i32 = 4;
const DEFAULT_MESSAGE_SPACING: i32 = 1;
const DEFAULT_PARTICIPANT_SPACING: i32 = 5;
const BOX_PADDING_LEFT_RIGHT: i32 = 2;
const MIN_BOX_WIDTH: i32 = 3;
const BOX_BORDER_WIDTH: i32 = 2;
const LABEL_LEFT_MARGIN: i32 = 2;
const LABEL_BUFFER_SPACE: i32 = 10;

#[derive(Debug)]
struct DiagramLayout {
    participant_widths: Vec<i32>,
    participant_centers: Vec<i32>,
    total_width: i32,
    message_spacing: i32,
    self_message_width: i32,
}

fn calculate_layout(diagram: &SequenceDiagram, config: &Config) -> DiagramLayout {
    let participant_spacing = if config.sequence_participant_spacing > 0 {
        config.sequence_participant_spacing
    } else {
        DEFAULT_PARTICIPANT_SPACING
    };

    let mut widths = Vec::with_capacity(diagram.participants.len());
    for participant in &diagram.participants {
        let label_width = UnicodeWidthStr::width(participant.label.as_str()) as i32;
        let mut w = label_width + BOX_PADDING_LEFT_RIGHT;
        if w < MIN_BOX_WIDTH {
            w = MIN_BOX_WIDTH;
        }
        widths.push(w);
    }

    let mut centers = Vec::with_capacity(diagram.participants.len());
    let mut current_x = 0;
    for width in &widths {
        let box_width = width + BOX_BORDER_WIDTH;
        if centers.is_empty() {
            centers.push(box_width / 2);
            current_x = box_width;
        } else {
            current_x += participant_spacing;
            centers.push(current_x + box_width / 2);
            current_x += box_width;
        }
    }

    let last = diagram.participants.len() - 1;
    let total_width = centers[last] + (widths[last] + BOX_BORDER_WIDTH) / 2;

    let message_spacing = if config.sequence_message_spacing > 0 {
        config.sequence_message_spacing
    } else {
        DEFAULT_MESSAGE_SPACING
    };

    let self_message_width = if config.sequence_self_message_width > 0 {
        config.sequence_self_message_width
    } else {
        DEFAULT_SELF_MESSAGE_WIDTH
    };

    DiagramLayout {
        participant_widths: widths,
        participant_centers: centers,
        total_width,
        message_spacing,
        self_message_width,
    }
}

pub fn render(diagram: &SequenceDiagram, config: &Config) -> Result<String, String> {
    if diagram.participants.is_empty() {
        return Err("no participants".to_string());
    }

    let chars = if config.use_ascii { ASCII } else { UNICODE };
    let layout = calculate_layout(diagram, config);

    let mut lines: Vec<String> = Vec::new();

    lines.push(build_line(diagram, &layout, |i| {
        let width = layout.participant_widths[i] as usize;
        format!(
            "{}{}{}",
            chars.top_left,
            chars.horizontal.to_string().repeat(width),
            chars.top_right
        )
    }));

    lines.push(build_line(diagram, &layout, |i| {
        let width = layout.participant_widths[i] as usize;
        let label = &diagram.participants[i].label;
        let label_len = UnicodeWidthStr::width(label.as_str()) as i32;
        let pad = ((width as i32 - label_len) / 2).max(0) as usize;
        let right_pad = width.saturating_sub(pad + label.chars().count());
        format!(
            "{}{}{}{}",
            chars.vertical,
            " ".repeat(pad),
            label,
            format!("{}{}", " ".repeat(right_pad), chars.vertical)
        )
    }));

    lines.push(build_line(diagram, &layout, |i| {
        let width = layout.participant_widths[i] as usize;
        let left = width / 2;
        let right = width - left - 1;
        format!(
            "{}{}{}{}{}",
            chars.bottom_left,
            chars.horizontal.to_string().repeat(left),
            chars.tee_down,
            chars.horizontal.to_string().repeat(right),
            chars.bottom_right
        )
    }));

    for message in &diagram.messages {
        for _ in 0..layout.message_spacing {
            lines.push(build_lifeline(&layout, chars));
        }

        if message.from == message.to {
            lines.extend(render_self_message(message, diagram, &layout, chars));
        } else {
            lines.extend(render_message(message, diagram, &layout, chars));
        }
    }

    lines.push(build_lifeline(&layout, chars));

    Ok(format!("{}\n", lines.join("\n")))
}

fn build_line<F>(diagram: &SequenceDiagram, layout: &DiagramLayout, draw: F) -> String
where
    F: Fn(usize) -> String,
{
    let mut out = String::new();
    for i in 0..diagram.participants.len() {
        let box_width = layout.participant_widths[i] + BOX_BORDER_WIDTH;
        let left = layout.participant_centers[i] - box_width / 2;
        let current_width = UnicodeWidthStr::width(out.as_str()) as i32;
        let needed = left - current_width;
        if needed > 0 {
            out.push_str(&" ".repeat(needed as usize));
        }
        out.push_str(&draw(i));
    }
    out
}

fn build_lifeline(layout: &DiagramLayout, chars: BoxChars) -> String {
    let mut line = vec![' '; (layout.total_width + 1) as usize];
    for center in &layout.participant_centers {
        let idx = *center as usize;
        if idx < line.len() {
            line[idx] = chars.vertical;
        }
    }
    rtrim(&line)
}

fn render_message(
    message: &Message,
    _diagram: &SequenceDiagram,
    layout: &DiagramLayout,
    chars: BoxChars,
) -> Vec<String> {
    let mut lines = Vec::new();
    let from = layout.participant_centers[message.from];
    let to = layout.participant_centers[message.to];

    let mut label = message.label.clone();
    if message.number > 0 {
        label = format!("{}. {}", message.number, label);
    }

    if !label.is_empty() {
        let start = i32::min(from, to) + LABEL_LEFT_MARGIN;
        let label_width = UnicodeWidthStr::width(label.as_str()) as i32;
        let mut line = build_lifeline(layout, chars).chars().collect::<Vec<char>>();
        let needed = (start + label_width + LABEL_BUFFER_SPACE) as usize;
        if line.len() < needed {
            line.resize(needed, ' ');
        }
        let mut col = start.max(0) as usize;
        for ch in label.chars() {
            if col < line.len() {
                line[col] = ch;
                col += 1;
            }
        }
        lines.push(rtrim(&line));
    }

    let mut line = build_lifeline(layout, chars).chars().collect::<Vec<char>>();
    let style = if matches!(message.arrow_type, ArrowType::Dotted) {
        chars.dotted_line
    } else {
        chars.solid_line
    };

    if from < to {
        line[from as usize] = chars.tee_right;
        for i in (from + 1)..to {
            line[i as usize] = style;
        }
        if (to - 1) >= 0 {
            line[(to - 1) as usize] = chars.arrow_right;
        }
        line[to as usize] = chars.vertical;
    } else {
        line[to as usize] = chars.vertical;
        line[(to + 1) as usize] = chars.arrow_left;
        for i in (to + 2)..from {
            line[i as usize] = style;
        }
        line[from as usize] = chars.tee_left;
    }
    lines.push(rtrim(&line));
    lines
}

fn render_self_message(
    message: &Message,
    _diagram: &SequenceDiagram,
    layout: &DiagramLayout,
    chars: BoxChars,
) -> Vec<String> {
    let mut lines = Vec::new();
    let center = layout.participant_centers[message.from] as usize;
    let width = layout.self_message_width as usize;

    let mut label = message.label.clone();
    if message.number > 0 {
        label = format!("{}. {}", message.number, label);
    }

    if !label.is_empty() {
        let mut line = ensure_width(
            build_lifeline(layout, chars),
            layout.total_width as usize + width + 1,
        );
        let start = center + LABEL_LEFT_MARGIN as usize;
        let label_width = UnicodeWidthStr::width(label.as_str()) as usize;
        let needed = start + label_width + LABEL_BUFFER_SPACE as usize;
        if line.len() < needed {
            line.resize(needed, ' ');
        }
        let mut col = start;
        for ch in label.chars() {
            if col < line.len() {
                line[col] = ch;
                col += 1;
            }
        }
        lines.push(rtrim(&line));
    }

    let mut l1 = ensure_width(
        build_lifeline(layout, chars),
        layout.total_width as usize + width + 1,
    );
    l1[center] = chars.tee_right;
    for i in 1..width {
        l1[center + i] = chars.horizontal;
    }
    l1[center + width - 1] = chars.self_top_right;
    lines.push(rtrim(&l1));

    let mut l2 = ensure_width(
        build_lifeline(layout, chars),
        layout.total_width as usize + width + 1,
    );
    l2[center + width - 1] = chars.vertical;
    lines.push(rtrim(&l2));

    let mut l3 = ensure_width(
        build_lifeline(layout, chars),
        layout.total_width as usize + width + 1,
    );
    l3[center] = chars.vertical;
    l3[center + 1] = chars.arrow_left;
    for i in 2..(width - 1) {
        l3[center + i] = chars.horizontal;
    }
    l3[center + width - 1] = chars.self_bottom;
    lines.push(rtrim(&l3));

    lines
}

fn ensure_width(line: String, width: usize) -> Vec<char> {
    let mut chars: Vec<char> = line.chars().collect();
    if chars.len() < width {
        chars.resize(width, ' ');
    }
    chars
}

fn rtrim(chars: &[char]) -> String {
    let mut end = chars.len();
    while end > 0 && chars[end - 1] == ' ' {
        end -= 1;
    }
    chars[..end].iter().collect()
}

impl SequenceDiagram {
    pub fn parse(&mut self, input: &str) -> Result<(), String> {
        *self = parse(input)?;
        Ok(())
    }

    pub fn render(&self, config: &Config) -> Result<String, String> {
        render(self, config)
    }
}

impl Diagram for SequenceDiagram {
    fn parse(&mut self, input: &str, _config: &Config) -> Result<(), String> {
        SequenceDiagram::parse(self, input)
    }

    fn render(&self, config: &Config) -> Result<String, String> {
        SequenceDiagram::render(self, config)
    }

    fn diagram_type(&self) -> &'static str {
        "sequence"
    }
}
