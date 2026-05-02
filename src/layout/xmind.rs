use super::*;

use std::collections::{BTreeMap, HashMap};

const ROOT_FONT_SIZE: f32 = 30.0;
const BRANCH_FONT_SIZE: f32 = 18.0;
const TOPIC_FONT_SIZE: f32 = 14.0;
const ROOT_PAD_X: f32 = 28.0;
const ROOT_PAD_Y: f32 = 14.0;
const BRANCH_PAD_X: f32 = 16.0;
const BRANCH_PAD_Y: f32 = 8.0;
const TOPIC_PAD_X: f32 = 6.0;
const TOPIC_PAD_Y: f32 = 7.0;
const ROOT_CHILD_GAP: f32 = 48.0;
const NODE_GAP: f32 = 24.0;
const EDGE_STUB: f32 = 12.0;
const LEAF_GAP: f32 = 40.0;
const ROOT_BRANCH_EXTRA_GAP: f32 = 26.0;
const RENDER_PAD: f32 = 24.0;
const ROOT_FILL: &str = "#28292d";
const ROOT_TEXT: &str = "#fdfcfd";
const TOPIC_TEXT: &str = "#0d0d0d";
const SECTION_COLORS: [&str; 4] = ["#ec662d", "#90c43c", "#f3cf4f", "#366ae5"];

#[derive(Clone)]
struct XmindNodeInfo {
    level: usize,
    section: Option<usize>,
    children: Vec<String>,
}

pub(super) fn compute_xmind_layout(graph: &Graph, theme: &Theme, _config: &LayoutConfig) -> Layout {
    let mut nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
    let mut info_map: HashMap<String, XmindNodeInfo> = HashMap::new();

    for node in &graph.mindmap.nodes {
        let label_text = graph
            .nodes
            .get(&node.id)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| node.label.clone());
        let is_root = node.level == 0;
        let font_size = xmind_font_size(node.level);
        let line_height = xmind_line_height(node.level);
        let label = xmind_label(&label_text, font_size, line_height, theme);
        let (pad_x, pad_y) = xmind_padding(node.level);
        let width = label.width + pad_x * 2.0;
        let height = label.height + pad_y * 2.0;
        let section = node.section.unwrap_or(0);
        let section_color = section_color(section);
        let mut style = crate::ir::NodeStyle::default();
        let shape = if is_root {
            style.fill = Some(ROOT_FILL.to_string());
            style.text_color = Some(ROOT_TEXT.to_string());
            style.stroke = Some("none".to_string());
            style.stroke_width = Some(0.0);
            crate::ir::NodeShape::RoundRect
        } else if node.level == 1 {
            style.fill = Some(section_color.to_string());
            style.text_color = Some(section_label_color(section_color).to_string());
            style.stroke = Some("none".to_string());
            style.stroke_width = Some(0.0);
            style.line_color = Some(section_color.to_string());
            crate::ir::NodeShape::Stadium
        } else {
            style.fill = Some("transparent".to_string());
            style.text_color = Some(TOPIC_TEXT.to_string());
            style.stroke = Some("none".to_string());
            style.stroke_width = Some(0.0);
            style.line_color = Some(section_color.to_string());
            crate::ir::NodeShape::Text
        };

        nodes.insert(
            node.id.clone(),
            NodeLayout {
                id: node.id.clone(),
                x: 0.0,
                y: 0.0,
                width,
                height,
                label,
                shape,
                style,
                link: graph.node_links.get(&node.id).cloned(),
                anchor_subgraph: None,
                hidden: false,
                icon: None,
            },
        );

        info_map.insert(
            node.id.clone(),
            XmindNodeInfo {
                level: node.level,
                section: node.section,
                children: node.children.clone(),
            },
        );
    }

    let root_id = graph
        .mindmap
        .root_id
        .clone()
        .or_else(|| graph.mindmap.nodes.first().map(|node| node.id.clone()));

    if let Some(root_id) = root_id.as_ref() {
        let mut y_centers = HashMap::new();
        y_centers.insert(root_id.clone(), 0.0);
        let root_children = info_map
            .get(root_id)
            .map(|info| info.children.clone())
            .unwrap_or_default();

        let mut cursor = 0.0;
        for (idx, child_id) in root_children.iter().enumerate() {
            assign_y_centers(child_id, &info_map, &mut y_centers, &mut cursor);
            if idx + 1 < root_children.len() {
                cursor += ROOT_BRANCH_EXTRA_GAP;
            }
        }
        if let (Some(first), Some(last)) = (root_children.first(), root_children.last()) {
            let shift = (y_centers.get(first).copied().unwrap_or(0.0)
                + y_centers.get(last).copied().unwrap_or(0.0))
                / 2.0;
            for center in y_centers.values_mut() {
                *center -= shift;
            }
            y_centers.insert(root_id.clone(), 0.0);
        }

        if let Some(root_node) = nodes.get_mut(root_id) {
            root_node.x = -root_node.width / 2.0;
            root_node.y = -root_node.height / 2.0;
        }
        assign_x_positions(root_id, &info_map, &mut nodes);

        for (id, node) in nodes.iter_mut() {
            let level = info_map.get(id).map(|info| info.level).unwrap_or(0);
            let center_y = y_centers.get(id).copied().unwrap_or(0.0);
            if level >= 2 {
                node.y = center_y - node.height;
            } else {
                node.y = center_y - node.height / 2.0;
            }
        }
    }

    let mut root_child_order: HashMap<String, usize> = HashMap::new();
    if let Some(root_id) = root_id.as_ref()
        && let Some(root_info) = info_map.get(root_id)
    {
        for (idx, child_id) in root_info.children.iter().enumerate() {
            root_child_order.insert(child_id.clone(), idx);
        }
    }

    let mut edges = Vec::new();
    for edge in &graph.edges {
        let Some(from_layout) = nodes.get(&edge.from) else {
            continue;
        };
        let Some(to_layout) = nodes.get(&edge.to) else {
            continue;
        };
        let from_info = info_map.get(&edge.from);
        let to_info = info_map.get(&edge.to);
        let section = to_info.and_then(|info| info.section).unwrap_or(0);
        let color = section_color(section).to_string();
        let from_level = from_info.map(|info| info.level).unwrap_or(0);
        let points = if from_level == 0 {
            let root_center_x = from_layout.x + from_layout.width / 2.0;
            let root_center_y = from_layout.y + from_layout.height / 2.0;
            let child_count = root_id
                .as_ref()
                .and_then(|id| info_map.get(id))
                .map(|info| info.children.len())
                .unwrap_or(1)
                .max(1);
            let child_idx = root_child_order.get(&edge.to).copied().unwrap_or(0);
            let closeness = center_closeness(child_idx, child_count);
            let start_x = root_center_x + from_layout.width * (0.122 + 0.211 * closeness);
            let end_y = xmind_anchor_y(to_layout, to_info);
            vec![
                (start_x, root_center_y),
                (start_x - 6.0, end_y),
                (to_layout.x, end_y),
            ]
        } else {
            let start_y = xmind_anchor_y(from_layout, from_info);
            let end_y = xmind_anchor_y(to_layout, to_info);
            let start = (from_layout.x + from_layout.width, start_y);
            let stub = (start.0 + EDGE_STUB, start.1);
            let bend = (stub.0, end_y);
            let end = (to_layout.x, end_y);
            vec![start, stub, bend, end]
        };
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points,
            directed: false,
            arrow_start: false,
            arrow_end: false,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride {
                stroke: Some(color),
                stroke_width: Some(if from_level == 0 { 3.0 } else { 2.0 }),
                dasharray: None,
                label_color: None,
            },
        });
    }

    let (width, height) = normalize_xmind_layout(&mut nodes, &mut edges);
    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs: Vec::new(),
        width,
        height,
        diagram: DiagramData::Graph {
            state_notes: Vec::new(),
        },
    }
}

fn assign_y_centers(
    node_id: &str,
    info_map: &HashMap<String, XmindNodeInfo>,
    y_centers: &mut HashMap<String, f32>,
    cursor: &mut f32,
) -> f32 {
    let children = info_map
        .get(node_id)
        .map(|info| info.children.clone())
        .unwrap_or_default();
    let center = if children.is_empty() {
        let value = *cursor;
        *cursor += LEAF_GAP;
        value
    } else {
        let mut first = 0.0;
        let mut last = 0.0;
        for (idx, child_id) in children.iter().enumerate() {
            let child_center = assign_y_centers(child_id, info_map, y_centers, cursor);
            if idx == 0 {
                first = child_center;
            }
            last = child_center;
        }
        (first + last) / 2.0
    };
    y_centers.insert(node_id.to_string(), center);
    center
}

fn assign_x_positions(
    node_id: &str,
    info_map: &HashMap<String, XmindNodeInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
    let Some(info) = info_map.get(node_id) else {
        return;
    };
    let Some(parent) = nodes.get(node_id).cloned() else {
        return;
    };
    for child_id in &info.children {
        let child_level = info_map.get(child_id).map(|child| child.level).unwrap_or(0);
        if let Some(child) = nodes.get_mut(child_id) {
            let gap = if child_level == 1 {
                ROOT_CHILD_GAP
            } else {
                NODE_GAP
            };
            child.x = parent.x + parent.width + gap;
        }
        assign_x_positions(child_id, info_map, nodes);
    }
}

fn normalize_xmind_layout(
    nodes: &mut BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
) -> (f32, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for node in nodes.values() {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    for edge in edges.iter() {
        for point in &edge.points {
            min_x = min_x.min(point.0);
            min_y = min_y.min(point.1);
            max_x = max_x.max(point.0);
            max_y = max_y.max(point.1);
        }
    }
    if min_x == f32::MAX || min_y == f32::MAX {
        return (1.0, 1.0);
    }
    let shift_x = RENDER_PAD - min_x;
    let shift_y = RENDER_PAD - min_y;
    for node in nodes.values_mut() {
        node.x += shift_x;
        node.y += shift_y;
    }
    for edge in edges {
        for point in &mut edge.points {
            point.0 += shift_x;
            point.1 += shift_y;
        }
    }
    (
        (max_x - min_x + RENDER_PAD * 2.0).max(1.0),
        (max_y - min_y + RENDER_PAD * 2.0).max(1.0),
    )
}

fn xmind_anchor_y(node: &NodeLayout, info: Option<&XmindNodeInfo>) -> f32 {
    if info.map(|info| info.level).unwrap_or(0) >= 2 {
        node.y + node.height
    } else {
        node.y + node.height / 2.0
    }
}

fn center_closeness(idx: usize, count: usize) -> f32 {
    if count <= 1 {
        return 1.0;
    }
    let mid = (count - 1) as f32 / 2.0;
    if mid <= 0.0 {
        return 1.0;
    }
    1.0 - ((idx as f32 - mid).abs() / mid)
}

fn xmind_font_size(level: usize) -> f32 {
    match level {
        0 => ROOT_FONT_SIZE,
        1 => BRANCH_FONT_SIZE,
        _ => TOPIC_FONT_SIZE,
    }
}

fn xmind_line_height(level: usize) -> f32 {
    match level {
        0 => 1.4,
        1 => 25.0 / BRANCH_FONT_SIZE,
        _ => 20.0 / TOPIC_FONT_SIZE,
    }
}

fn xmind_padding(level: usize) -> (f32, f32) {
    match level {
        0 => (ROOT_PAD_X, ROOT_PAD_Y),
        1 => (BRANCH_PAD_X, BRANCH_PAD_Y),
        _ => (TOPIC_PAD_X, TOPIC_PAD_Y),
    }
}

fn xmind_label(text: &str, font_size: f32, line_height: f32, theme: &Theme) -> TextBlock {
    let lines = split_xmind_lines(text);
    let cjk_factor = if font_size >= ROOT_FONT_SIZE {
        0.95
    } else {
        1.03
    };
    let width = lines
        .iter()
        .map(|line| xmind_text_width(line, font_size, cjk_factor))
        .fold(0.0, f32::max);
    let height = lines.len().max(1) as f32 * font_size * line_height;
    let _ = theme;
    TextBlock {
        lines,
        width,
        height,
    }
}

fn split_xmind_lines(text: &str) -> Vec<String> {
    let normalized = text.replace("<br/>", "\n").replace("<br>", "\n");
    let mut lines: Vec<String> = normalized
        .split('\n')
        .map(|line| line.trim().to_string())
        .collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn xmind_text_width(text: &str, font_size: f32, cjk_factor: f32) -> f32 {
    text.chars()
        .map(|ch| {
            if is_cjk(ch) {
                font_size * cjk_factor
            } else if ch.is_ascii_whitespace() {
                font_size * 0.32
            } else if ch.is_ascii_punctuation() {
                font_size * 0.36
            } else if ch.is_ascii() {
                font_size * 0.56
            } else {
                font_size * cjk_factor
            }
        })
        .sum()
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{4dbf}'
            | '\u{4e00}'..='\u{9fff}'
            | '\u{f900}'..='\u{faff}'
            | '\u{20000}'..='\u{2a6df}'
            | '\u{2a700}'..='\u{2b73f}'
            | '\u{2b740}'..='\u{2b81f}'
            | '\u{2b820}'..='\u{2ceaf}'
    )
}

fn section_color(section: usize) -> &'static str {
    SECTION_COLORS[section % SECTION_COLORS.len()]
}

fn section_label_color(fill: &str) -> &'static str {
    match fill {
        "#90c43c" | "#f3cf4f" => "#061406",
        _ => "#fafcfa",
    }
}
