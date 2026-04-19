use crate::dashboard::buffer::{CellStyle, GameColor, PlayfieldBuffer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenePoint {
    pub x: f32,
    pub y: f32,
}

impl ScenePoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneSize {
    pub width: f32,
    pub height: f32,
}

impl SceneSize {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl SceneRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    pub fn inset(self, dx: f32, dy: f32) -> Self {
        let width = (self.width - dx * 2.0).max(0.0);
        let height = (self.height - dy * 2.0).max(0.0);
        Self::new(self.x + dx, self.y + dy, width, height)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextNode {
    pub text: String,
    pub origin: ScenePoint,
    pub style: CellStyle,
    pub clip: Option<SceneRect>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadNode {
    pub rect: SceneRect,
    pub color: GameColor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineNode {
    pub start: ScenePoint,
    pub end: ScenePoint,
    pub thickness: f32,
    pub color: GameColor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretNode {
    pub rect: SceneRect,
    pub color: GameColor,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SceneNode {
    Text(TextNode),
    Quad(QuadNode),
    Line(LineNode),
    Caret(CaretNode),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneGraph {
    logical_size: SceneSize,
    nodes: Vec<SceneNode>,
}

impl SceneGraph {
    pub fn new(logical_width: f32, logical_height: f32) -> Self {
        Self {
            logical_size: SceneSize::new(logical_width, logical_height),
            nodes: Vec::new(),
        }
    }

    pub fn logical_size(&self) -> SceneSize {
        self.logical_size
    }

    pub fn nodes(&self) -> &[SceneNode] {
        &self.nodes
    }

    pub fn push_node(&mut self, node: SceneNode) {
        self.nodes.push(node);
    }

    pub fn push_quad(&mut self, rect: SceneRect, color: GameColor) {
        self.push_node(SceneNode::Quad(QuadNode { rect, color }));
    }

    pub fn push_line(
        &mut self,
        start: ScenePoint,
        end: ScenePoint,
        thickness: f32,
        color: GameColor,
    ) {
        self.push_node(SceneNode::Line(LineNode {
            start,
            end,
            thickness,
            color,
        }));
    }

    pub fn push_text(
        &mut self,
        origin: ScenePoint,
        text: impl Into<String>,
        style: CellStyle,
        clip: Option<SceneRect>,
    ) {
        self.push_node(SceneNode::Text(TextNode {
            text: text.into(),
            origin,
            style,
            clip,
        }));
    }

    pub fn push_caret(&mut self, rect: SceneRect, color: GameColor) {
        self.push_node(SceneNode::Caret(CaretNode { rect, color }));
    }
}

#[derive(Debug)]
pub enum UiScene {
    Playfield(PlayfieldBuffer),
    Graph(SceneGraph),
}

impl UiScene {
    pub fn from_playfield(playfield: PlayfieldBuffer) -> Self {
        Self::Playfield(playfield)
    }

    pub fn graph(logical_width: f32, logical_height: f32) -> Self {
        Self::Graph(SceneGraph::new(logical_width, logical_height))
    }

    pub fn as_playfield(&self) -> Option<&PlayfieldBuffer> {
        match self {
            Self::Playfield(playfield) => Some(playfield),
            Self::Graph(_) => None,
        }
    }

    pub fn as_graph(&self) -> Option<&SceneGraph> {
        match self {
            Self::Playfield(_) => None,
            Self::Graph(graph) => Some(graph),
        }
    }

    pub fn as_graph_mut(&mut self) -> Option<&mut SceneGraph> {
        match self {
            Self::Playfield(_) => None,
            Self::Graph(graph) => Some(graph),
        }
    }

    pub fn into_playfield(self) -> Option<PlayfieldBuffer> {
        match self {
            Self::Playfield(playfield) => Some(playfield),
            Self::Graph(_) => None,
        }
    }
}

impl From<PlayfieldBuffer> for UiScene {
    fn from(playfield: PlayfieldBuffer) -> Self {
        Self::from_playfield(playfield)
    }
}
