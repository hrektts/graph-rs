use std::marker::PhantomData;
use std::ops::Deref;
use slab::{self, Slab};

use graph::{AdjacencyGraph, AdjacencyMatrixGraph, BidirectionalGraph, EdgeDescriptor,
            EdgeListGraph, Directivity, FromUsize, Graph, IncidenceGraph, MutableGraph,
            VertexDescriptor, VertexListGraph};

#[derive(Clone, Debug)]
pub struct IncidenceList<D, VP, EP> {
    vertices: Slab<Vertex<VP>>,
    edges: Slab<Edge<EP>>,
    phantom: PhantomData<D>,
}

#[derive(Clone, Debug, Hash)]
pub struct Vertex<VP> {
    incidence: (Option<EdgeDescriptor>, VP, Option<EdgeDescriptor>),
}

impl<VP> Deref for Vertex<VP> {
    type Target = (Option<EdgeDescriptor>, VP, Option<EdgeDescriptor>);

    fn deref(&self) -> &Self::Target {
        &self.incidence
    }
}

#[derive(Clone, Debug, Hash)]
pub struct Edge<EP> {
    incidence: (Option<VertexDescriptor>, EP, Option<VertexDescriptor>),
    next: (Option<EdgeDescriptor>, Option<EdgeDescriptor>),
}

impl<EP> Deref for Edge<EP> {
    type Target = (Option<VertexDescriptor>, EP, Option<VertexDescriptor>);

    fn deref(&self) -> &Self::Target {
        &self.incidence
    }
}

impl<D, VP, EP> IncidenceList<D, VP, EP> {
    pub fn new() -> Self {
        Self {
            vertices: Slab::new(),
            edges: Slab::new(),
            phantom: PhantomData,
        }
    }

    pub fn with_order(order: usize) -> Self {
        Self {
            vertices: Slab::with_capacity(order),
            edges: Slab::new(),
            phantom: PhantomData,
        }
    }

    pub fn with_order_size(order: usize, size: usize) -> Self {
        Self {
            vertices: Slab::with_capacity(order),
            edges: Slab::with_capacity(size),
            phantom: PhantomData,
        }
    }

    pub fn with_size(size: usize) -> Self {
        Self {
            vertices: Slab::new(),
            edges: Slab::with_capacity(size),
            phantom: PhantomData,
        }
    }
}

impl<D, VP, EP> Graph for IncidenceList<D, VP, EP> {
    type Directivity = D;
    type VertexProperty = VP;
    type EdgeProperty = EP;

    fn vertex_property(&self, d: VertexDescriptor) -> Option<&Self::VertexProperty> {
        self.vertices.get(d.into()).and_then(|&Vertex {
             incidence: (_, ref vp, _),
         }| Some(vp))
    }

    fn edge_property(&self, d: EdgeDescriptor) -> Option<&Self::EdgeProperty> {
        self.edges.get(d.into()).and_then(|&Edge {
             incidence: (_, ref ep, _),
             next: _,
         }| Some(ep))
    }
}

impl<'a, D, VP, EP> IncidenceGraph<'a> for IncidenceList<D, VP, EP>
where
    D: 'a,
    VP: 'a,
    EP: 'a,
{
    type Incidences = IncidentEdges<'a, D, VP, EP>;

    fn out_degree(&self, d: VertexDescriptor) -> usize {
        self.out_edges(d).fold(0, |acc, _| acc + 1)
    }

    fn out_edges(&'a self, d: VertexDescriptor) -> Self::Incidences {
        let &(_, _, oe) = self.vertices[d.into()].deref();
        IncidentEdges {
            graph: self,
            current_edge_descriptor: oe,
            kind: EdgeKind::Outgoing,
        }
    }

    fn source(&self, d: EdgeDescriptor) -> VertexDescriptor {
        let &(s, _, _) = self.edges[d.into()].deref();
        assert!(s.is_some());
        s.unwrap()
    }

    fn target(&self, d: EdgeDescriptor) -> VertexDescriptor {
        let &(_, _, t) = self.edges[d.into()].deref();
        assert!(t.is_some());
        t.unwrap()
    }
}

#[derive(Clone, Debug, Hash)]
pub enum EdgeKind {
    Outgoing,
    Incoming,
}

#[derive(Clone, Debug)]
pub struct IncidentEdges<'a, D, VP, EP>
where
    D: 'a,
    VP: 'a,
    EP: 'a,
{
    graph: &'a IncidenceList<D, VP, EP>,
    current_edge_descriptor: Option<EdgeDescriptor>,
    kind: EdgeKind,
}

impl<'a, D, VP, EP> Iterator for IncidentEdges<'a, D, VP, EP> {
    type Item = EdgeDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_edge_descriptor {
            None => None,
            Some(ed) => {
                self.graph.edges.get(ed.into()).and_then(|e| {
                    let &Edge {
                        incidence: _,
                        next: (ie, oe),
                    } = e;
                    match self.kind {
                        EdgeKind::Outgoing => self.current_edge_descriptor = oe,
                        EdgeKind::Incoming => self.current_edge_descriptor = ie,
                    }
                    Some(ed)
                })
            }
        }
    }
}

impl<'a, D, VP, EP> BidirectionalGraph<'a> for IncidenceList<D, VP, EP>
where
    D: 'a,
    VP: 'a,
    EP: 'a,
{
    fn degree(&self, d: VertexDescriptor) -> usize {
        self.out_edges(d.clone()).chain(self.in_edges(d)).fold(
            0,
            |acc, _| {
                acc + 1
            },
        )
    }
    fn in_degree(&self, d: VertexDescriptor) -> usize {
        self.in_edges(d).fold(0, |acc, _| acc + 1)
    }

    fn in_edges(&'a self, d: VertexDescriptor) -> Self::Incidences {
        let &(ie, _, _) = self.vertices[d.into()].deref();
        IncidentEdges {
            graph: self,
            current_edge_descriptor: ie,
            kind: EdgeKind::Incoming,
        }
    }
}

impl<'a, D, VP, EP> AdjacencyGraph<'a> for IncidenceList<D, VP, EP>
where
    D: Directivity,
{
    type Adjacencies = Box<Iterator<Item = VertexDescriptor> + 'a>;

    fn adjacent_vertices(&'a self, d: VertexDescriptor) -> Self::Adjacencies {
        let &(ie, _, oe) = self.vertices[d.into()].deref();
        let successors = IncidentVertices {
            graph: self,
            current_edge_descriptor: oe,
            kind: VertexKind::Successor,
        };
        let predecessors = IncidentVertices {
            graph: self,
            current_edge_descriptor: ie,
            kind: VertexKind::Predecessor,
        };

        let mut vs = if D::is_directed() {
            successors.collect::<Vec<_>>()
        } else {
            successors.chain(predecessors).collect::<Vec<_>>()
        };
        vs.sort();
        vs.dedup();
        Box::new(vs.into_iter())
    }
}

#[derive(Clone, Debug, Hash)]
pub enum VertexKind {
    Predecessor,
    Successor,
}

#[derive(Clone, Debug)]
pub struct IncidentVertices<'a, D, VP, EP>
where
    D: 'a,
    VP: 'a,
    EP: 'a,
{
    graph: &'a IncidenceList<D, VP, EP>,
    current_edge_descriptor: Option<EdgeDescriptor>,
    kind: VertexKind,
}

impl<'a, D, VP, EP> Iterator for IncidentVertices<'a, D, VP, EP> {
    type Item = VertexDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_edge_descriptor {
            None => None,
            Some(ed) => {
                self.graph.edges.get(ed.into()).and_then(|e| {
                    let &Edge {
                        incidence: (s, _, t),
                        next: (ie, oe),
                    } = e;
                    match self.kind {
                        VertexKind::Predecessor => {
                            self.current_edge_descriptor = ie;
                            s
                        }
                        VertexKind::Successor => {
                            self.current_edge_descriptor = oe;
                            t
                        }
                    }
                })
            }
        }
    }
}

impl<'a, D, VP, EP> VertexListGraph<'a> for IncidenceList<D, VP, EP>
where
    VP: 'a,
{
    type Vertices = ::std::iter::Map<
        slab::Iter<'a, Vertex<VP>>,
        fn((usize, &Vertex<VP>)) -> VertexDescriptor,
    >;

    fn order(&self) -> usize {
        self.vertices.len()
    }

    fn vertices(&'a self) -> Self::Vertices {
        self.vertices.iter().map(
            |(k, _)| VertexDescriptor::from_usize(k),
        )
    }
}

impl<'a, D, VP, EP> EdgeListGraph<'a> for IncidenceList<D, VP, EP>
where
    EP: 'a,
{
    type Edges = ::std::iter::Map<slab::Iter<'a, Edge<EP>>, fn((usize, &Edge<EP>)) -> EdgeDescriptor>;

    fn size(&self) -> usize {
        self.edges.len()
    }

    fn edges(&'a self) -> Self::Edges {
        self.edges.iter().map(
            |(k, _)| EdgeDescriptor::from_usize(k),
        )
    }
}

impl<D, VP, EP> AdjacencyMatrixGraph for IncidenceList<D, VP, EP>
where
    D: Directivity,
{
    fn edge(&self, source: VertexDescriptor, target: VertexDescriptor) -> Option<EdgeDescriptor> {
        self.edges
            .iter()
            .find(|&(_,
               &Edge {
                   incidence: (s, _, t),
                   next: _,
               })| {
                (s == Some(source) && t == Some(target)) ||
                    (!D::is_directed() && s == Some(target) && t == Some(source))
            })
            .and_then(|(k, _)| Some(EdgeDescriptor::from_usize(k)))
    }
}

impl<D, VP, EP> MutableGraph for IncidenceList<D, VP, EP> {
    fn add_vertex(&mut self, property: Self::VertexProperty) -> VertexDescriptor {
        let k = self.vertices.insert(
            Vertex { incidence: (None, property, None) },
        );
        VertexDescriptor::from_usize(k)
    }

    fn add_edge(
        &mut self,
        source: VertexDescriptor,
        target: VertexDescriptor,
        property: Self::EdgeProperty,
    ) -> Option<EdgeDescriptor> {
        let entry = self.edges.vacant_entry();
        let key = entry.key();
        let oe = self.vertices.get_mut(source.into()).and_then(
            |&mut Vertex {
                 incidence: (_, _, ref mut oe),
             }| {
                let next_oe = *oe;
                *oe = Some(EdgeDescriptor::from_usize(key));
                Some(next_oe)
            },
        );
        let ie = match oe {
            None => None,
            Some(_) => {
                self.vertices.get_mut(target.into()).and_then(
                    |&mut Vertex {
                         incidence: (ref mut ie, _, _),
                     }| {
                        let next_ie = *ie;
                        *ie = Some(EdgeDescriptor::from_usize(key));
                        Some(next_ie)
                    },
                )
            }
        };

        if oe.is_some() && ie.is_some() {
            let edge = Edge {
                incidence: (Some(source.into()), property, Some(target.into())),
                next: (ie.unwrap(), oe.unwrap()),
            };
            entry.insert(edge);
            Some(EdgeDescriptor::from_usize(key))
        } else {
            None
        }
    }

    fn remove_vertex(&mut self, d: VertexDescriptor) -> Option<Self::VertexProperty> {
        if self.vertices.contains(d.into()) {
            let eds = self.out_edges(d.into())
                .chain(self.in_edges(d.into()))
                .collect::<Vec<_>>();
            for ed in eds {
                if self.remove_edge(ed).is_none() {
                    return None;
                }
            }

            let Vertex { incidence: (_, vp, _) } = self.vertices.remove(d.into());
            Some(vp)
        } else {
            None
        }
    }

    fn remove_edge(&mut self, d: EdgeDescriptor) -> Option<Self::EdgeProperty> {
        if let Some((s, t, ie, oe)) =
            self.edges.get(d.into()).and_then(|&Edge {
                 incidence: (s, _, t),
                 next: (ie, oe),
             }| Some((s, t, ie, oe)))
        {
            s.and_then(|vd| {
                let done = {
                    let &mut Vertex { incidence: (_, _, ref mut oe_to_check) } =
                        self.vertices.get_mut(vd.into()).unwrap();
                    oe_to_check.and_then(|x| {
                        if x == d {
                            *oe_to_check = oe;
                        }
                        Some(())
                    })
                };
                done.or_else(|| {
                    self.out_edges(vd).find(|&x| x == d).and_then(|ed| {
                        let &mut Edge {
                            incidence: _,
                            next: (_, ref mut oe_to_change),
                        } = &mut self.edges[ed.into()];
                        *oe_to_change = oe;
                        Some(())
                    })
                })
            });

            t.and_then(|vd| {
                let done = {
                    let &mut Vertex { incidence: (ref mut ie_to_check, _, _) } =
                        self.vertices.get_mut(vd.into()).unwrap();
                    ie_to_check.and_then(|x| {
                        if x == d {
                            *ie_to_check = ie;
                        }
                        Some(())
                    })
                };
                done.or_else(|| {
                    self.in_edges(vd).find(|&x| x == d).and_then(|ed| {
                        let &mut Edge {
                            incidence: _,
                            next: (ref mut ie_to_change, _),
                        } = &mut self.edges[ed.into()];
                        *ie_to_change = ie;
                        Some(())
                    })
                })
            });

            let Edge {
                incidence: (_, ep, _),
                next: _,
            } = self.edges.remove(d.into());
            Some(ep)
        } else {
            None
        }
    }

    fn vertex_property_mut(&mut self, d: VertexDescriptor) -> Option<&mut Self::VertexProperty> {
        self.vertices.get_mut(d.into()).and_then(|&mut Vertex {
             incidence: (_, ref mut vp, _),
         }| Some(vp))
    }

    fn edge_property_mut(&mut self, d: EdgeDescriptor) -> Option<&mut Self::EdgeProperty> {
        self.edges.get_mut(d.into()).and_then(|&mut Edge {
             incidence: (_, ref mut ep, _),
             next: _,
         }| Some(ep))
    }
}

#[cfg(test)]
mod tests {
    use super::IncidenceList;

    #[test]
    fn vertex_attribute() {
        use graph::{Graph, Directed, VertexListGraph, MutableGraph};

        let mut g = IncidenceList::<Directed, usize, String>::new();

        let v1 = g.add_vertex(42);
        let v2 = g.add_vertex(13);
        let v3 = g.add_vertex(1337);

        assert!(g.vertices().any(|x| {
            (v1 != x) ^ (g.vertex_property(x) == Some(&42))
        }));
        assert!(g.vertices().any(|x| {
            (v2 != x) ^ (g.vertex_property(x) == Some(&13))
        }));
        assert!(g.vertices().any(|x| {
            (v3 != x) ^ (g.vertex_property(x) == Some(&1337))
        }));
        assert!(g.vertices().any(|x| g.vertex_property(x) != Some(&69)));
    }

    #[test]
    fn general_usage() {
        use graph::{Directed, EdgeListGraph, Graph, IncidenceGraph, MutableGraph, VertexListGraph};

        let mut g = IncidenceList::<Directed, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);

        let e12 = g.add_edge(v1, v2, "a".into());
        let e23 = g.add_edge(v2, v3, "b".into());
        let e31 = g.add_edge(v3, v1, "c".into());

        // V1 <--E31--- V3
        // |            ^
        // E12          |
        // |            |
        // v            |
        // V2 ---E23----+

        assert!(e12.is_some() && e23.is_some() && e31.is_some());
        assert!(v1 != v2);
        assert!(v1 != v3);
        assert!(v2 != v3);

        assert!(e12 != e23);
        assert!(e12 != e31);
        assert!(e23 != e31);

        assert!(g.vertex_property(v1) == Some(&3));
        assert!(g.vertex_property(v2) == Some(&5));
        assert!(g.vertex_property(v3) == Some(&7));

        assert!(g.edge_property(e12.unwrap()) == Some(&"a".to_string()));
        assert!(g.edge_property(e23.unwrap()) == Some(&"b".to_string()));
        assert!(g.edge_property(e31.unwrap()) == Some(&"c".to_string()));

        assert_eq!(g.size(), 3);
        assert_eq!(g.order(), 3);

        assert_eq!(g.source(e12.unwrap()), v1);
        assert_eq!(g.target(e12.unwrap()), v2);
        assert_eq!(g.source(e23.unwrap()), v2);
        assert_eq!(g.target(e23.unwrap()), v3);
        assert_eq!(g.source(e31.unwrap()), v3);
        assert_eq!(g.target(e31.unwrap()), v1);

        assert_eq!(g.out_degree(v1), 1);
        assert_eq!(g.out_degree(v2), 1);
        assert_eq!(g.out_degree(v3), 1);

        assert!(g.remove_edge(e12.unwrap()).is_some());

        assert!(g.remove_vertex(v1).is_some());
        assert!(g.remove_vertex(v2).is_some());
        assert!(g.remove_vertex(v3).is_some());

        assert_eq!(g.order(), 0);
        assert_eq!(g.size(), 0);
    }

    #[test]
    fn degree() {
        use graph::{Directed, IncidenceGraph, BidirectionalGraph, EdgeListGraph, MutableGraph};

        let mut g = IncidenceList::<Directed, Option<isize>, String>::new();

        let v1 = g.add_vertex(Some(3));
        let v2 = g.add_vertex(None);
        let v3 = g.add_vertex(Some(3));

        assert!(g.add_edge(v1, v2, "a".into()) != None);
        let e23 = g.add_edge(v2, v3, "b".into());
        assert!(e23.is_some());
        assert!(g.add_edge(v3, v1, "c".into()) != None);

        // V1 <------- V3
        // |           ^
        // |           |
        // v           |
        // V2 ---E23---+

        assert_eq!(g.out_degree(v1), 1);
        assert_eq!(g.out_degree(v2), 1);
        assert_eq!(g.out_degree(v3), 1);

        assert_eq!(g.in_degree(v1), 1);
        assert_eq!(g.in_degree(v2), 1);
        assert_eq!(g.in_degree(v3), 1);

        let v4 = g.add_vertex(Some(3));
        assert!(g.add_edge(v4, v1, "d".to_string()) != None);

        // V4
        // |
        // |
        // v
        // V1 <------- V3
        // |           ^
        // |           |
        // v           |
        // V2 ---E23---+

        assert_eq!(g.in_degree(v1), 2);
        assert_eq!(g.in_degree(v2), 1);
        assert_eq!(g.in_degree(v3), 1);
        assert_eq!(g.in_degree(v4), 0);

        assert_eq!(g.out_degree(v1), 1);
        assert_eq!(g.out_degree(v2), 1);
        assert_eq!(g.out_degree(v3), 1);
        assert_eq!(g.out_degree(v4), 1);

        assert!(g.remove_edge(e23.unwrap()).is_some());
        g.add_edge(v3, v2, "d1".to_string());
        let v5 = g.add_vertex(None);
        g.add_edge(v2, v5, "d2".to_string());
        g.add_edge(v5, v3, "d3".to_string());
        g.add_edge(v5, v4, "d4".to_string());

        // V4 <-------------------+
        // |                      |
        // |                      |
        // v                      |
        // V1 <------- V3 <-------+
        // |           |          |
        // |           |          |
        // v           |          |
        // V2 <--------+          |
        // |                      |
        // |                      |
        // +--------------------> V5

        assert_eq!(g.in_degree(v1), 2);
        assert_eq!(g.in_degree(v2), 2);
        assert_eq!(g.in_degree(v3), 1);
        assert_eq!(g.in_degree(v4), 1);
        assert_eq!(g.in_degree(v5), 1);

        assert_eq!(g.out_degree(v1), 1);
        assert_eq!(g.out_degree(v2), 1);
        assert_eq!(g.out_degree(v3), 2);
        assert_eq!(g.out_degree(v4), 1);
        assert_eq!(g.out_degree(v5), 2);

        assert_eq!(g.size(), 7);
    }

    #[test]
    fn out_iterator() {
        use graph::{Directed, IncidenceGraph, MutableGraph};

        let mut g = IncidenceList::<Directed, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);
        let v4 = g.add_vertex(11);

        let e12 = g.add_edge(v1, v2, "a".into());
        let e23 = g.add_edge(v2, v3, "b".into());
        let e21 = g.add_edge(v2, v1, "c".into());
        let e14 = g.add_edge(v1, v4, "d".into());

        // +--> V1 ---E14--> V4
        // |    |
        // E21  E12
        // |    |
        // |    v
        // +--- V2 ---E23--> V3

        assert!(e12.is_some() && e23.is_some() && e21.is_some() && e14.is_some());

        let i = g.out_edges(v1).collect::<Vec<_>>();
        assert!(i == vec![e12.unwrap(), e14.unwrap()] || i == vec![e14.unwrap(), e12.unwrap()]);

        let i = g.out_edges(v2).collect::<Vec<_>>();
        assert!(i == vec![e23.unwrap(), e21.unwrap()] || i == vec![e21.unwrap(), e23.unwrap()]);

        assert_eq!(g.out_edges(v3).next(), None);
        assert_eq!(g.out_edges(v4).next(), None);
    }

    #[test]
    fn in_iterator() {
        use graph::{BidirectionalGraph, Directed, MutableGraph};

        let mut g = IncidenceList::<Directed, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);
        let v4 = g.add_vertex(11);

        let e12 = g.add_edge(v1, v2, "a".into());
        let e23 = g.add_edge(v2, v3, "b".into());
        let e21 = g.add_edge(v2, v1, "c".into());
        let e14 = g.add_edge(v1, v4, "d".into());

        // +--> V1 ---E14--> V4
        // |    |
        // E21  E12
        // |    |
        // |    v
        // +--- V2 ---E23--> V3

        assert!(e12.is_some() && e23.is_some() && e21.is_some() && e14.is_some());

        let i = g.in_edges(v1).collect::<Vec<_>>();
        assert!(i == vec![e21.unwrap()]);

        let i = g.in_edges(v2).collect::<Vec<_>>();
        assert!(i == vec![e12.unwrap()]);

        let i = g.in_edges(v3).collect::<Vec<_>>();
        assert!(i == vec![e23.unwrap()]);

        let i = g.in_edges(v4).collect::<Vec<_>>();
        assert!(i == vec![e14.unwrap()]);
    }

    #[test]
    fn adj_iterator_on_directed_graph() {
        use graph::{AdjacencyGraph, Directed, MutableGraph};

        let mut g = IncidenceList::<Directed, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);
        let v4 = g.add_vertex(11);

        g.add_edge(v1, v2, "a".into());
        g.add_edge(v2, v3, "b".into());
        g.add_edge(v2, v1, "c".into());
        g.add_edge(v1, v4, "d".into());

        // +--> V1 ---E14--> V4
        // |    |
        // E21  E12
        // |    |
        // |    v
        // +--- V2 ---E23--> V3

        let i = g.adjacent_vertices(v1).collect::<Vec<_>>();
        assert!(i == vec![v2, v4] || i == vec![v4, v2]);

        let i = g.adjacent_vertices(v2).collect::<Vec<_>>();
        assert!(i == vec![v1, v3] || i == vec![v3, v1]);

        let i = g.adjacent_vertices(v3).collect::<Vec<_>>();
        assert!(i == vec![]);

        let i = g.adjacent_vertices(v4).collect::<Vec<_>>();
        assert!(i == vec![]);
    }

    #[test]
    fn adj_iterator_on_undirected_graph() {
        use graph::{AdjacencyGraph, MutableGraph, Undirected};

        let mut g = IncidenceList::<Undirected, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);
        let v4 = g.add_vertex(11);

        g.add_edge(v1, v2, "a".into());
        g.add_edge(v2, v3, "b".into());
        g.add_edge(v2, v1, "c".into());
        g.add_edge(v1, v4, "d".into());

        // +--- V1 ---E14--- V4
        // |    |
        // E21  E12
        // |    |
        // +--- V2 ---E23--- V3

        let i = g.adjacent_vertices(v1).collect::<Vec<_>>();
        assert!(i == vec![v2, v4] || i == vec![v4, v2]);

        let i = g.adjacent_vertices(v2).collect::<Vec<_>>();
        assert!(i == vec![v1, v3] || i == vec![v3, v1]);

        let i = g.adjacent_vertices(v3).collect::<Vec<_>>();
        assert!(i == vec![v2]);

        let i = g.adjacent_vertices(v4).collect::<Vec<_>>();
        assert!(i == vec![v1]);
    }

    #[test]
    fn vertices_edges_iterators() {
        use graph::{Directed, EdgeListGraph, MutableGraph, VertexListGraph};
        use std::collections::HashSet;

        let mut g = IncidenceList::<Directed, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);
        let v4 = g.add_vertex(11);

        let e12 = g.add_edge(v1, v2, "a".into());
        let e23 = g.add_edge(v2, v3, "b".into());
        let e21 = g.add_edge(v2, v1, "c".into());
        let e14 = g.add_edge(v1, v4, "d".into());

        // +--> V1 ---E14--> V4
        // |    |
        // E21  E12
        // |    |
        // |    v
        // +--- V2 ---E23--> V3

        assert!(e12.is_some() && e23.is_some() && e21.is_some() && e14.is_some());

        let vs = g.vertices().collect::<HashSet<_>>();
        assert!(vs.contains(&v1) && vs.contains(&v2) && vs.contains(&v3) && vs.contains(&v4));
        assert_eq!(vs.len(), 4);

        let es = g.edges().collect::<HashSet<_>>();
        assert!(
            es.contains(&e12.unwrap()) && es.contains(&e23.unwrap()) &&
                es.contains(&e21.unwrap()) &&
                es.contains(&e14.unwrap())
        );
        assert_eq!(es.len(), 4);
    }

    #[test]
    fn duplicate_label() {
        use graph::{EdgeListGraph, Directed, MutableGraph, VertexListGraph};

        let mut g = IncidenceList::<Directed, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);

        let e12 = g.add_edge(v1, v2, "a".into());
        let e23 = g.add_edge(v2, v3, "b".into());

        // V1          V3
        // |           ^
        // |           |
        // v           |
        // V2 ---------+

        assert!(e12.is_some() && e23.is_some());

        assert_eq!(g.order(), 3);
        assert_eq!(g.size(), 2);
    }

    #[test]
    fn remove_edge_from_node_with_multiple_out_edges() {
        use graph::{Directed, EdgeListGraph, IncidenceGraph, MutableGraph, VertexListGraph};

        let mut g = IncidenceList::<Directed, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);

        let e12 = g.add_edge(v1, v2, "a".into());
        let e13 = g.add_edge(v1, v3, "b".into());

        // V1 -------> V3
        // |
        // |
        // v
        // V2

        assert!(e12.is_some() && e13.is_some());

        assert_eq!(g.size(), 2);
        assert_eq!(g.order(), 3);

        assert!(g.remove_edge(e12.unwrap()).is_some());

        // V1 -------> V3
        //
        //
        //
        // V2

        assert_eq!(g.out_degree(v1), 1);
        assert_eq!(g.size(), 1);
        assert_eq!(g.order(), 3);
    }

    #[test]
    fn edge_on_directed_graph() {
        use graph::{AdjacencyMatrixGraph, Directed, MutableGraph};

        let mut g = IncidenceList::<Directed, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);
        let v4 = g.add_vertex(11);

        let e12 = g.add_edge(v1, v2, "a".into());
        let e23 = g.add_edge(v2, v3, "b".into());
        let e21 = g.add_edge(v2, v1, "c".into());
        let e14 = g.add_edge(v1, v4, "d".into());

        // +--> V1 ---E14--> V4
        // |    |
        // E21  E12
        // |    |
        // |    v
        // +--- V2 ---E23--> V3

        assert!(e12.is_some() && e23.is_some() && e21.is_some() && e14.is_some());

        assert_eq!(g.edge(v1, v1), None);
        assert_eq!(g.edge(v1, v2), e12);
        assert_eq!(g.edge(v1, v3), None);
        assert_eq!(g.edge(v1, v4), e14);
        assert_eq!(g.edge(v2, v1), e21);
        assert_eq!(g.edge(v2, v2), None);
        assert_eq!(g.edge(v2, v3), e23);
        assert_eq!(g.edge(v2, v4), None);
        assert_eq!(g.edge(v3, v1), None);
        assert_eq!(g.edge(v3, v2), None);
        assert_eq!(g.edge(v3, v3), None);
        assert_eq!(g.edge(v3, v4), None);
        assert_eq!(g.edge(v4, v1), None);
        assert_eq!(g.edge(v4, v2), None);
        assert_eq!(g.edge(v4, v3), None);
        assert_eq!(g.edge(v4, v4), None);
    }

    #[test]
    fn edge_on_undirected_graph() {
        use graph::{AdjacencyMatrixGraph, MutableGraph, Undirected};

        let mut g = IncidenceList::<Undirected, isize, String>::new();

        let v1 = g.add_vertex(3);
        let v2 = g.add_vertex(5);
        let v3 = g.add_vertex(7);
        let v4 = g.add_vertex(11);

        let e12 = g.add_edge(v1, v2, "a".into());
        let e23 = g.add_edge(v2, v3, "b".into());
        let e22 = g.add_edge(v2, v2, "c".into());
        let e14 = g.add_edge(v1, v4, "d".into());

        //   V1 ---E14--- V4
        //   |
        //   E12
        //   |
        //   V2 ---E23--- V3
        //  / \
        // /   \
        // \   /
        //  E22

        assert!(e12.is_some() && e23.is_some() && e22.is_some() && e14.is_some());

        assert_eq!(g.edge(v1, v1), None);
        assert_eq!(g.edge(v1, v2), e12);
        assert_eq!(g.edge(v1, v3), None);
        assert_eq!(g.edge(v1, v4), e14);
        assert_eq!(g.edge(v2, v1), e12);
        assert_eq!(g.edge(v2, v2), e22);
        assert_eq!(g.edge(v2, v3), e23);
        assert_eq!(g.edge(v2, v4), None);
        assert_eq!(g.edge(v3, v1), None);
        assert_eq!(g.edge(v3, v2), e23);
        assert_eq!(g.edge(v3, v3), None);
        assert_eq!(g.edge(v3, v4), None);
        assert_eq!(g.edge(v4, v1), e14);
        assert_eq!(g.edge(v4, v2), None);
        assert_eq!(g.edge(v4, v3), None);
        assert_eq!(g.edge(v4, v4), None);
    }
}
