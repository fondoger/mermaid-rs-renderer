//! Intermediate representation (IR) for parsed Mermaid diagrams.
//!
//! This module defines the data structures produced by the parser and consumed
//! by the layout engine. A [`Graph`] is the top-level container that holds all
//! nodes, edges, subgraphs, and diagram-specific data for every supported
//! Mermaid diagram type.

use std::collections::{BTreeMap, HashMap};

/// Layout direction for flowcharts and state diagrams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Top-to-bottom (aliases: `TD`, `TB`).
    TopDown,
    /// Left-to-right (`LR`).
    LeftRight,
    /// Bottom-to-top (`BT`).
    BottomTop,
    /// Right-to-left (`RL`).
    RightLeft,
}

/// The kind of Mermaid diagram being represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramKind {
    Flowchart,
    Class,
    State,
    Sequence,
    Er,
    Pie,
    Mindmap,
    Journey,
    Timeline,
    Gantt,
    Requirement,
    GitGraph,
    C4,
    Sankey,
    Quadrant,
    ZenUML,
    Block,
    Packet,
    Kanban,
    Architecture,
    Radar,
    Treemap,
    XYChart,
}

/// The type of combined fragment in a sequence diagram (e.g. `alt`, `loop`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceFrameKind {
    Alt,
    Opt,
    Loop,
    Par,
    Rect,
    Critical,
    Break,
}

/// Where a note is placed relative to a sequence diagram participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceNotePosition {
    LeftOf,
    RightOf,
    Over,
}

/// Where a note is placed relative to a state diagram state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateNotePosition {
    LeftOf,
    RightOf,
}

/// Whether a sequence participant's activation box starts or ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceActivationKind {
    Activate,
    Deactivate,
}

/// An activation or deactivation event on a sequence diagram lifeline.
#[derive(Debug, Clone)]
pub struct SequenceActivation {
    /// The participant whose lifeline is affected.
    pub participant: String,
    /// The message index at which this event occurs.
    pub index: usize,
    pub kind: SequenceActivationKind,
}

/// A note attached to one or more participants in a sequence diagram.
#[derive(Debug, Clone)]
pub struct SequenceNote {
    pub position: SequenceNotePosition,
    /// The participant(s) the note is associated with.
    pub participants: Vec<String>,
    pub label: String,
    /// The message index where this note appears.
    pub index: usize,
}

/// A single slice in a pie chart.
#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f32,
}

/// A data point plotted on a quadrant chart.
#[derive(Debug, Clone)]
pub struct QuadrantPoint {
    pub label: String,
    /// Normalised x-coordinate (0.0–1.0).
    pub x: f32,
    /// Normalised y-coordinate (0.0–1.0).
    pub y: f32,
}

/// Status tag on a Gantt chart task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GanttStatus {
    Done,
    Active,
    Crit,
    Milestone,
}

/// Parsed data for a quadrant chart.
#[derive(Debug, Clone, Default)]
pub struct QuadrantData {
    pub title: Option<String>,
    pub x_axis_left: Option<String>,
    pub x_axis_right: Option<String>,
    pub y_axis_bottom: Option<String>,
    pub y_axis_top: Option<String>,
    /// Labels for quadrants in order: top-right, top-left, bottom-left, bottom-right.
    pub quadrant_labels: [Option<String>; 4],
    pub points: Vec<QuadrantPoint>,
}

/// A single task bar in a Gantt chart.
#[derive(Debug, Clone)]
pub struct GanttTask {
    pub id: String,
    pub label: String,
    /// Absolute start date/time string (e.g. `"2024-01-15"`).
    pub start: Option<String>,
    /// Duration string (e.g. `"3d"`, `"1w"`).
    pub duration: Option<String>,
    /// ID of a task this one should start after.
    pub after: Option<String>,
    /// Section this task belongs to.
    pub section: Option<String>,
    pub status: Option<GanttStatus>,
}

/// Visual style of a commit in a git graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitGraphCommitType {
    Normal,
    Reverse,
    Highlight,
    Merge,
    CherryPick,
}

/// A single commit in a git graph diagram.
#[derive(Debug, Clone)]
pub struct GitGraphCommit {
    pub id: String,
    pub message: Option<String>,
    /// Sequential index among all commits.
    pub seq: usize,
    pub commit_type: GitGraphCommitType,
    /// Overridden visual type from `type:` syntax.
    pub custom_type: Option<GitGraphCommitType>,
    pub tags: Vec<String>,
    /// Parent commit IDs.
    pub parents: Vec<String>,
    /// Branch this commit belongs to.
    pub branch: String,
    /// Whether the commit has a user-supplied id (vs auto-generated).
    pub custom_id: bool,
}

/// A branch definition in a git graph diagram.
#[derive(Debug, Clone)]
pub struct GitGraphBranch {
    pub name: String,
    /// Explicit ordering value (lower = drawn first).
    pub order: Option<f32>,
    /// Insertion order for stable sorting.
    pub insertion_index: usize,
}

/// Aggregated git graph data extracted from the diagram source.
#[derive(Debug, Clone, Default)]
pub struct GitGraphData {
    pub main_branch: String,
    pub commits: Vec<GitGraphCommit>,
    pub branches: Vec<GitGraphBranch>,
}

/// Shape kind for C4 architecture model elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum C4ShapeKind {
    Person,
    ExternalPerson,
    System,
    SystemDb,
    SystemQueue,
    ExternalSystem,
    ExternalSystemDb,
    ExternalSystemQueue,
    Container,
    ContainerDb,
    ContainerQueue,
    ExternalContainer,
    ExternalContainerDb,
    ExternalContainerQueue,
    Component,
    ComponentDb,
    ComponentQueue,
    ExternalComponent,
    ExternalComponentDb,
    ExternalComponentQueue,
}

impl C4ShapeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            C4ShapeKind::Person => "person",
            C4ShapeKind::ExternalPerson => "external_person",
            C4ShapeKind::System => "system",
            C4ShapeKind::SystemDb => "system_db",
            C4ShapeKind::SystemQueue => "system_queue",
            C4ShapeKind::ExternalSystem => "external_system",
            C4ShapeKind::ExternalSystemDb => "external_system_db",
            C4ShapeKind::ExternalSystemQueue => "external_system_queue",
            C4ShapeKind::Container => "container",
            C4ShapeKind::ContainerDb => "container_db",
            C4ShapeKind::ContainerQueue => "container_queue",
            C4ShapeKind::ExternalContainer => "external_container",
            C4ShapeKind::ExternalContainerDb => "external_container_db",
            C4ShapeKind::ExternalContainerQueue => "external_container_queue",
            C4ShapeKind::Component => "component",
            C4ShapeKind::ComponentDb => "component_db",
            C4ShapeKind::ComponentQueue => "component_queue",
            C4ShapeKind::ExternalComponent => "external_component",
            C4ShapeKind::ExternalComponentDb => "external_component_db",
            C4ShapeKind::ExternalComponentQueue => "external_component_queue",
        }
    }
}

/// A C4 model element (person, system, container, or component).
#[derive(Debug, Clone)]
pub struct C4Shape {
    pub id: String,
    pub label: String,
    pub type_label: Option<String>,
    /// Technology descriptor (e.g. `"Spring Boot"`).
    pub techn: Option<String>,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    /// ID of the enclosing boundary (empty string for top-level).
    pub parent_boundary: String,
    pub kind: C4ShapeKind,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
}

/// A C4 boundary container (e.g. `System_Boundary`, `Container_Boundary`).
#[derive(Debug, Clone)]
pub struct C4Boundary {
    pub id: String,
    pub label: String,
    pub boundary_type: String,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    /// ID of the enclosing boundary (empty string for top-level).
    pub parent_boundary: String,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
}

/// Direction/kind of a C4 relationship arrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C4RelKind {
    Rel,
    BiRel,
    RelUp,
    RelDown,
    RelLeft,
    RelRight,
    RelBack,
}

/// A relationship (arrow) between two C4 elements.
#[derive(Debug, Clone)]
pub struct C4Rel {
    pub kind: C4RelKind,
    pub from: String,
    pub to: String,
    pub label: String,
    pub techn: Option<String>,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub offset_x: f32,
    pub offset_y: f32,
    pub line_color: Option<String>,
    pub text_color: Option<String>,
}

/// All parsed C4 architecture data for a diagram.
#[derive(Debug, Clone, Default)]
pub struct C4Data {
    pub shapes: Vec<C4Shape>,
    pub boundaries: Vec<C4Boundary>,
    pub rels: Vec<C4Rel>,
    /// The C4 diagram level (e.g. `"C4Context"`, `"C4Container"`).
    pub c4_type: Option<String>,
    pub c4_shape_in_row_override: Option<usize>,
    pub c4_boundary_in_row_override: Option<usize>,
}

/// A `box` grouping around a set of sequence diagram participants.
#[derive(Debug, Clone)]
pub struct SequenceBox {
    pub label: Option<String>,
    pub color: Option<String>,
    pub participants: Vec<String>,
}

/// All parsed data specific to sequence diagrams.
#[derive(Debug, Clone, Default)]
pub struct SequenceData {
    /// Ordered participant IDs.
    pub participants: Vec<String>,
    pub frames: Vec<SequenceFrame>,
    pub notes: Vec<SequenceNote>,
    pub activations: Vec<SequenceActivation>,
    /// Starting number for `autonumber`, if enabled.
    pub autonumber: Option<usize>,
    pub boxes: Vec<SequenceBox>,
}

/// All parsed data specific to pie chart diagrams.
#[derive(Debug, Clone, Default)]
pub struct PieData {
    pub slices: Vec<PieSlice>,
    pub title: Option<String>,
    pub show_data: bool,
}

/// All parsed data specific to Gantt chart diagrams.
#[derive(Debug, Clone, Default)]
pub struct GanttData {
    pub tasks: Vec<GanttTask>,
    pub title: Option<String>,
    pub sections: Vec<String>,
}

/// A note annotation attached to a state diagram state.
#[derive(Debug, Clone)]
pub struct StateNote {
    pub position: StateNotePosition,
    /// The state ID this note is attached to.
    pub target: String,
    pub label: String,
}

/// One section inside a sequence diagram combined fragment (e.g. one `else` branch).
#[derive(Debug, Clone)]
pub struct SequenceFrameSection {
    pub label: Option<String>,
    /// First message index covered by this section.
    pub start_idx: usize,
    /// Last message index covered by this section.
    pub end_idx: usize,
}

/// A combined fragment (`alt`, `loop`, `opt`, etc.) in a sequence diagram.
#[derive(Debug, Clone)]
pub struct SequenceFrame {
    pub kind: SequenceFrameKind,
    pub sections: Vec<SequenceFrameSection>,
    /// First message index covered by the entire frame.
    pub start_idx: usize,
    /// Last message index covered by the entire frame.
    pub end_idx: usize,
}

impl Direction {
    pub fn from_token(token: &str) -> Option<Self> {
        let upper = token.to_ascii_uppercase();
        match upper.as_str() {
            "TD" | "TB" => Some(Self::TopDown),
            "BT" => Some(Self::BottomTop),
            "LR" => Some(Self::LeftRight),
            "RL" => Some(Self::RightLeft),
            _ => None,
        }
    }
}

/// A node (vertex) in the diagram graph.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    /// Display text rendered inside the node shape.
    pub label: String,
    pub shape: NodeShape,
    /// Optional numeric value (used by journey tasks for the score).
    pub value: Option<f32>,
}

/// A clickable hyperlink attached to a node via the `click` directive.
#[derive(Debug, Clone)]
pub struct NodeLink {
    pub url: String,
    pub title: Option<String>,
    /// Link target attribute (e.g. `"_blank"`).
    pub target: Option<String>,
}

/// A directed or undirected edge between two nodes.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Label rendered along the middle of the edge.
    pub label: Option<String>,
    /// Label rendered near the source end (ER/class multiplicity).
    pub start_label: Option<String>,
    /// Label rendered near the target end (ER/class multiplicity).
    pub end_label: Option<String>,
    pub directed: bool,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub arrow_start_kind: Option<EdgeArrowhead>,
    pub arrow_end_kind: Option<EdgeArrowhead>,
    pub start_decoration: Option<EdgeDecoration>,
    pub end_decoration: Option<EdgeDecoration>,
    pub style: EdgeStyle,
}

/// Line style for edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    /// A continuous line (`-->`).
    Solid,
    /// A dashed/dotted line (`-.->`).
    Dotted,
    /// A thick line (`==>`).
    Thick,
}

/// Decorative marker drawn at the start or end of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDecoration {
    Circle,
    Cross,
    Diamond,
    DiamondFilled,
    /// Crow's foot: exactly one (`||`).
    CrowsFootOne,
    /// Crow's foot: zero or one (`o|`).
    CrowsFootZeroOne,
    /// Crow's foot: one or many (`|{`).
    CrowsFootMany,
    /// Crow's foot: zero or many (`o{`).
    CrowsFootZeroMany,
}

/// Arrowhead variant for specialised edges (class dependency, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeArrowhead {
    OpenTriangle,
    ClassDependency,
}

/// A subgraph (cluster) containing a set of nodes.
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: Option<String>,
    pub label: String,
    /// IDs of nodes that belong to this subgraph.
    pub nodes: Vec<String>,
    /// Optional direction override for nodes inside this subgraph.
    pub direction: Option<Direction>,
}

/// Top-level intermediate representation of any parsed Mermaid diagram.
///
/// A single `Graph` is produced by the parser and consumed by the layout
/// engine. Because different diagram types carry different data, many fields
/// are only populated for the corresponding [`DiagramKind`] and are empty or
/// `None` for other kinds.
#[derive(Debug, Clone)]
pub struct Graph {
    /// Which Mermaid diagram type this graph represents.
    pub kind: DiagramKind,
    /// Primary layout direction (used by flowcharts, state diagrams, etc.).
    pub direction: Direction,
    /// Nodes keyed by their ID (insertion-order preserved via `BTreeMap`).
    pub nodes: BTreeMap<String, Node>,
    /// Maps node ID → insertion order index for deterministic iteration.
    pub node_order: HashMap<String, usize>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,

    // -- Diagram-specific data (grouped into sub-structs) --
    pub sequence: SequenceData,
    pub pie: PieData,
    pub gantt: GanttData,

    // -- State diagram data --
    pub state_notes: Vec<StateNote>,

    // -- Quadrant chart data --
    pub quadrant: QuadrantData,

    // -- Journey diagram data --
    pub journey_title: Option<String>,

    // -- Git graph data --
    pub gitgraph: GitGraphData,

    // -- Styling --
    /// Named style classes defined via `classDef`.
    pub class_defs: HashMap<String, NodeStyle>,
    /// Maps node ID → list of applied class names.
    pub node_classes: HashMap<String, Vec<String>>,
    /// Per-node inline style overrides.
    pub node_styles: HashMap<String, NodeStyle>,
    pub subgraph_styles: HashMap<String, NodeStyle>,
    pub subgraph_classes: HashMap<String, Vec<String>>,
    /// Clickable links attached to nodes.
    pub node_links: HashMap<String, NodeLink>,
    /// Per-edge style overrides keyed by edge index.
    pub edge_styles: HashMap<usize, EdgeStyleOverride>,
    /// Default style override applied to all edges.
    pub edge_style_default: Option<EdgeStyleOverride>,

    // -- Other diagram-specific data --
    pub c4: C4Data,
    pub mindmap: MindmapData,
    pub xychart: XYChartData,
    pub timeline: TimelineData,
    pub block: Option<BlockDiagram>,
}

/// The geometric shape used to render a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    Rectangle,
    /// Horizontal bar used for fork/join states.
    ForkJoin,
    RoundRect,
    /// Rounded rectangle with fully-rounded ends (`([…])`).
    Stadium,
    /// Double-bordered rectangle (`[[…]]`).
    Subroutine,
    Cylinder,
    /// Actor-style box used by sequence diagram participants.
    ActorBox,
    Circle,
    DoubleCircle,
    Diamond,
    Hexagon,
    Parallelogram,
    /// Reversed slant parallelogram.
    ParallelogramAlt,
    Trapezoid,
    /// Inverted trapezoid.
    TrapezoidAlt,
    /// Flag/asymmetric shape (`>…]`).
    Asymmetric,
    /// Default shape for mindmap nodes.
    MindmapDefault,
    /// Plain text with no border.
    Text,
}

/// Shape variant for mindmap nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindmapNodeType {
    Default,
    RoundedRect,
    Rect,
    Circle,
    Cloud,
    Bang,
    Hexagon,
}

/// A node in the mindmap tree.
#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub id: String,
    pub label: String,
    /// Indentation level (0 = root).
    pub level: usize,
    /// Section index for colour cycling.
    pub section: Option<usize>,
    pub node_type: MindmapNodeType,
    pub icon: Option<String>,
    pub class: Option<String>,
    /// IDs of direct children in the tree.
    pub children: Vec<String>,
}

/// Parsed mindmap tree data.
#[derive(Debug, Clone, Default)]
pub struct MindmapData {
    pub nodes: Vec<MindmapNode>,
    pub root_id: Option<String>,
}

/// The plot type for an XY chart data series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XYSeriesKind {
    Bar,
    Line,
}

/// A data series in an XY chart.
#[derive(Debug, Clone)]
pub struct XYSeries {
    pub kind: XYSeriesKind,
    pub label: Option<String>,
    pub values: Vec<f32>,
}

/// Parsed XY chart data.
#[derive(Debug, Clone, Default)]
pub struct XYChartData {
    pub title: Option<String>,
    pub x_axis_label: Option<String>,
    pub x_axis_categories: Vec<String>,
    pub y_axis_label: Option<String>,
    pub y_axis_min: Option<f32>,
    pub y_axis_max: Option<f32>,
    pub series: Vec<XYSeries>,
}

/// A single time period and its associated events in a timeline diagram.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub time: String,
    pub events: Vec<String>,
    pub section: Option<String>,
}

/// Parsed timeline diagram data.
#[derive(Debug, Clone, Default)]
pub struct TimelineData {
    pub title: Option<String>,
    pub events: Vec<TimelineEvent>,
    pub sections: Vec<String>,
}

/// Parsed block diagram data.
#[derive(Debug, Clone, Default)]
pub struct BlockDiagram {
    /// Number of columns in the block grid layout.
    pub columns: Option<usize>,
    pub nodes: Vec<BlockNode>,
}

/// A node in a block diagram grid.
#[derive(Debug, Clone)]
pub struct BlockNode {
    pub id: String,
    /// How many grid columns this block spans.
    pub span: usize,
    /// If true this is an empty spacer rather than a visible block.
    pub is_space: bool,
}

impl Graph {
    /// Create a new graph for the given diagram kind.
    pub fn with_kind(kind: DiagramKind) -> Self {
        let mut g = Self::new();
        g.kind = kind;
        g
    }

    pub fn new() -> Self {
        Self {
            kind: DiagramKind::Flowchart,
            direction: Direction::TopDown,
            nodes: BTreeMap::new(),
            node_order: HashMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            sequence: SequenceData::default(),
            pie: PieData::default(),
            gantt: GanttData::default(),
            state_notes: Vec::new(),
            quadrant: QuadrantData::default(),
            journey_title: None,
            gitgraph: GitGraphData::default(),
            class_defs: HashMap::new(),
            node_classes: HashMap::new(),
            node_styles: HashMap::new(),
            subgraph_styles: HashMap::new(),
            subgraph_classes: HashMap::new(),
            node_links: HashMap::new(),
            edge_styles: HashMap::new(),
            edge_style_default: None,
            c4: C4Data::default(),
            mindmap: MindmapData::default(),
            xychart: XYChartData::default(),
            timeline: TimelineData::default(),
            block: None,
        }
    }

    pub fn ensure_node(&mut self, id: &str, label: Option<String>, shape: Option<NodeShape>) {
        let is_new = !self.nodes.contains_key(id);
        let entry = self.nodes.entry(id.to_string()).or_insert(Node {
            id: id.to_string(),
            label: id.to_string(),
            shape: NodeShape::Rectangle,
            value: None,
        });
        if is_new {
            let order = self.node_order.len();
            self.node_order.insert(id.to_string(), order);
        }
        if let Some(label) = label {
            entry.label = label;
        }
        if let Some(shape) = shape {
            entry.shape = shape;
        }
    }
}

/// Visual style overrides for a node, populated via `classDef` or `style` directives.
#[derive(Debug, Clone, Default)]
pub struct NodeStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub text_color: Option<String>,
    pub stroke_width: Option<f32>,
    pub stroke_dasharray: Option<String>,
    pub line_color: Option<String>,
}

/// Per-edge style overrides applied via `linkStyle` directives.
#[derive(Debug, Clone, Default)]
pub struct EdgeStyleOverride {
    pub stroke: Option<String>,
    pub stroke_width: Option<f32>,
    pub dasharray: Option<String>,
    pub label_color: Option<String>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
