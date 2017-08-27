pub trait FromUsize {
    fn from_usize(v: usize) -> Self;
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VertexDescriptor(usize);

impl From<VertexDescriptor> for usize {
    fn from(v: VertexDescriptor) -> Self {
        v.0
    }
}

impl FromUsize for VertexDescriptor {
    fn from_usize(v: usize) -> Self {
        VertexDescriptor(v)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeDescriptor(usize);

impl From<EdgeDescriptor> for usize {
    fn from(v: EdgeDescriptor) -> Self {
        v.0
    }
}

impl FromUsize for EdgeDescriptor {
    fn from_usize(v: usize) -> Self {
        EdgeDescriptor(v)
    }
}

pub trait Graph {
    type Directivity;
    type VertexProperty;
    type EdgeProperty;

    fn vertex_property(&self, d: VertexDescriptor) -> Option<&Self::VertexProperty>;
    fn edge_property(&self, d: EdgeDescriptor) -> Option<&Self::EdgeProperty>;
}

pub trait IncidenceGraph<'a>: Graph {
    type Incidences: Iterator<Item = EdgeDescriptor>;

    fn out_degree(&self, d: VertexDescriptor) -> usize;
    fn out_edges(&'a self, d: VertexDescriptor) -> Self::Incidences;
    fn source(&self, d: EdgeDescriptor) -> VertexDescriptor;
    fn target(&self, d: EdgeDescriptor) -> VertexDescriptor;
}

pub trait BidirectionalGraph<'a>: IncidenceGraph<'a> {
    fn degree(&self, d: VertexDescriptor) -> usize;
    fn in_degree(&self, d: VertexDescriptor) -> usize;
    fn in_edges(&'a self, d: VertexDescriptor) -> Self::Incidences;
}

pub trait AdjacencyGraph<'a>: Graph {
    type Adjacencies: Iterator<Item = VertexDescriptor>;

    fn adjacent_vertices(&'a self, d: VertexDescriptor) -> Self::Adjacencies;
}

pub trait VertexListGraph<'a>: Graph {
    type Vertices: Iterator<Item = VertexDescriptor>;

    fn order(&self) -> usize;
    fn vertices(&'a self) -> Self::Vertices;
}

pub trait EdgeListGraph<'a>: Graph {
    type Edges: Iterator<Item = EdgeDescriptor>;

    fn size(&self) -> usize;
    fn edges(&'a self) -> Self::Edges;
}

pub trait AdjacencyMatrixGraph: Graph {
    fn edge(&self, source: VertexDescriptor, target: VertexDescriptor) -> Option<EdgeDescriptor>;
}

pub trait MutableGraph: Graph {
    fn add_vertex(&mut self, property: Self::VertexProperty) -> VertexDescriptor;
    fn add_edge(
        &mut self,
        source: VertexDescriptor,
        target: VertexDescriptor,
        property: Self::EdgeProperty,
    ) -> Option<EdgeDescriptor>;
    fn remove_vertex(&mut self, d: VertexDescriptor) -> Option<Self::VertexProperty>;
    fn remove_edge(&mut self, d: EdgeDescriptor) -> Option<Self::EdgeProperty>;
    fn vertex_property_mut(&mut self, d: VertexDescriptor) -> Option<&mut Self::VertexProperty>;
    fn edge_property_mut(&mut self, d: EdgeDescriptor) -> Option<&mut Self::EdgeProperty>;
}

pub trait Directivity {
    fn is_directed() -> bool;
}

#[derive(Clone, Copy, Debug)]
pub struct Directed;

impl Directivity for Directed {
    fn is_directed() -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Undirected;

impl Directivity for Undirected {
    fn is_directed() -> bool {
        false
    }
}
