use graph::{Graph, EdgeDescriptor, VertexDescriptor};

pub trait Visitor<G, T>
where
    G: Graph,
{
    fn visit(&mut self, e: &T, graph: &G);
}

pub enum Event {
    InitializeVertex(VertexDescriptor),
    StartVertex(VertexDescriptor),
    DiscoverVertex(VertexDescriptor),
    FinishVertex(VertexDescriptor),
    ExamineVertex(VertexDescriptor),
    ExamineEdge(EdgeDescriptor),
    TreeEdge(EdgeDescriptor),
    NonTreeEdge(EdgeDescriptor),
    GrayTarget(EdgeDescriptor),
    BlackTarget(EdgeDescriptor),
    ForwardOrCrossEdge(EdgeDescriptor),
    BackEdge(EdgeDescriptor),
    FinishEdge(EdgeDescriptor),
    EdgeRelaxed(EdgeDescriptor),
    EdgeNotRelaxed(EdgeDescriptor),
    EdgeMinimized(EdgeDescriptor),
    EdgeNotMinimized(EdgeDescriptor),
}

pub struct DefaultVisitor;

impl<G> Visitor<G, Event> for DefaultVisitor
where
    G: Graph,
{
    fn visit(&mut self, _e: &Event, _g: &G) {}
}
