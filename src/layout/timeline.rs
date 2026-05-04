use super::*;

pub(super) fn compute_timeline_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let data = &graph.timeline;
    let font_size = theme.font_size;
    let event_font_size = theme.font_size * 0.9;
    let padding = 30.0_f32;
    let event_width = 120.0_f32;
    let min_event_height = 80.0_f32;
    let event_spacing = 40.0_f32;
    let event_text_width = (event_width - 16.0).max(1.0);
    let title_height = if data.title.is_some() { 40.0 } else { 0.0 };
    let line_y = padding + title_height + 60.0;

    let num_events = data.events.len().max(1);
    let total_events_width =
        num_events as f32 * event_width + (num_events - 1) as f32 * event_spacing;

    let width = padding * 2.0 + total_events_width;

    let title = data.title.as_ref().map(|t| measure_label(t, theme, config));

    let mut max_event_height = min_event_height;
    let mut events = Vec::with_capacity(data.events.len());
    for (i, event) in data.events.iter().enumerate() {
        let x = padding + i as f32 * (event_width + event_spacing);
        let y = line_y + 30.0;

        let time_block = measure_label_with_font_size(
            &event.time,
            font_size,
            config,
            false,
            theme.font_family.as_str(),
        );
        let event_blocks: Vec<TextBlock> = event
            .events
            .iter()
            .map(|e| {
                measure_label_with_max_width(
                    e,
                    event_font_size,
                    event_text_width,
                    config,
                    true,
                    theme.font_family.as_str(),
                )
            })
            .collect();

        let time_extra =
            time_block.lines.len().saturating_sub(1) as f32 * font_size * config.label_line_height;
        let description_height = event_blocks
            .iter()
            .map(|block| block.lines.len() as f32 * event_font_size * config.label_line_height)
            .sum::<f32>();
        let event_height = (40.0 + time_extra + description_height + 16.0).max(min_event_height);
        max_event_height = max_event_height.max(event_height);

        events.push(TimelineEventLayout {
            time: time_block,
            events: event_blocks,
            x,
            y,
            width: event_width,
            height: event_height,
            circle_y: line_y,
        });
    }

    let height = padding * 2.0 + title_height + max_event_height + 100.0;
    let line_start_x = padding;
    let line_end_x = width - padding;

    // Sections (simplified - just record them)
    let sections: Vec<TimelineSectionLayout> = data
        .sections
        .iter()
        .enumerate()
        .map(|(i, section)| {
            let label = measure_label(section, theme, config);
            TimelineSectionLayout {
                label,
                x: padding + i as f32 * 200.0,
                y: padding,
                width: 180.0,
                height: 30.0,
            }
        })
        .collect();

    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__timeline_metrics_content".to_string(),
        NodeLayout {
            id: "__timeline_metrics_content".to_string(),
            x: padding,
            y: padding,
            width: (width - padding * 2.0).max(1.0),
            height: (height - padding * 2.0).max(1.0),
            label: TextBlock {
                lines: vec![String::new()],
                width: 0.0,
                height: 0.0,
            },
            shape: crate::ir::NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
        },
    );

    Layout {
        kind: graph.kind,
        nodes,
        edges: Vec::new(),
        subgraphs: Vec::new(),
        diagram: DiagramData::Timeline(TimelineLayout {
            title,
            title_y: padding + font_size,
            events,
            sections,
            line_y,
            line_start_x,
            line_end_x,
            width,
            height,
        }),
        width,
        height,
    }
}
