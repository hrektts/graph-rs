use std::cmp::Ordering;
use std::fmt::Debug;
use std::collections::BinaryHeap;
use std::collections::hash_map::Entry;
use std::marker::PhantomData;

use fnv::FnvHashMap;
use num_traits::Zero;

use graph::{Graph, AdjacencyGraph, AdjacencyMatrixGraph, VertexListGraph, EdgeDescriptor,
            VertexDescriptor};
use path::reverse_path;
use visitor::{Event, Visitor, DefaultVisitor};

#[derive(Clone, Eq, Debug)]
struct State<C>
where
    C: Ord,
{
    evaluation: C,
    cost: C,
    vertex: VertexDescriptor,
}


impl<C> PartialEq for State<C>
where
    C: Ord,
{
    fn eq(&self, other: &Self) -> bool {
        self.evaluation == other.evaluation
    }
}

impl<C> PartialOrd for State<C>
where
    C: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<C> Ord for State<C>
where
    C: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        other.evaluation.cmp(&self.evaluation)
    }
}

pub struct Astar<C, T, V>
where
    C: Copy + Debug + Ord + Zero,
    T: Graph,
    V: Visitor<T, Event>,
{
    fringe: BinaryHeap<State<C>>,
    parents: FnvHashMap<VertexDescriptor, (VertexDescriptor, C)>,
    visitor: V,
    phantom: PhantomData<T>,
}

impl<C, T> Astar<C, T, DefaultVisitor>
where
    C: Copy + Debug + Ord + Zero,
    T: Graph,
{
    pub fn new() -> Self {
        Self::with_visitor(DefaultVisitor)
    }
}

impl<C, T, V> Astar<C, T, V>
where
    C: Copy + Debug + Ord + Zero,
    T: Graph,
    V: Visitor<T, Event>,
{
    pub fn with_visitor(visitor: V) -> Self {
        Self {
            fringe: BinaryHeap::new(),
            parents: FnvHashMap::default(),
            visitor: visitor,
            phantom: PhantomData,
        }
    }

    pub fn run<'a, F, G, H>(
        &mut self,
        start: &VertexDescriptor,
        edge_cost: G,
        heuristic: H,
        is_goal: F,
        graph: &'a T,
    ) -> Option<Vec<VertexDescriptor>>
    where
        C: Copy + Debug + Ord + Zero,
        F: Fn(&VertexDescriptor) -> bool,
        G: Fn(&EdgeDescriptor, &T) -> C,
        H: Fn(&VertexDescriptor, &T) -> C,
        T: AdjacencyGraph<'a> + AdjacencyMatrixGraph + VertexListGraph<'a>,
    {
        for vertex in graph.vertices() {
            self.visitor.visit(&Event::InitializeVertex(vertex), graph)
        }

        self.visitor.visit(&Event::DiscoverVertex(*start), graph);
        self.fringe.push(State {
            evaluation: heuristic(start, graph),
            cost: C::zero(),
            vertex: *start,
        });

        while let Some(State { cost, vertex, .. }) = self.fringe.pop() {
            self.visitor.visit(&Event::ExamineVertex(vertex), graph);
            if is_goal(&vertex) {
                let parents = self.parents.iter().map(|(&n, &(p, _))| (n, p)).collect();
                return Some(reverse_path(&parents, vertex));
            }
            for adjacency in graph.adjacent_vertices(vertex) {
                let edge = graph.edge(vertex, adjacency).unwrap();
                self.visitor.visit(&Event::ExamineEdge(edge), graph);
                let cost_to_adjacency = cost + edge_cost(&edge, graph);
                if adjacency != *start {
                    match self.parents.entry(adjacency) {
                        Entry::Vacant(entry) => {
                            entry.insert((vertex, cost_to_adjacency));
                            self.visitor.visit(&Event::EdgeRelaxed(edge), graph);
                            self.visitor.visit(&Event::DiscoverVertex(adjacency), graph);
                            self.fringe.push(State {
                                evaluation: cost_to_adjacency + heuristic(&adjacency, graph),
                                cost: cost_to_adjacency,
                                vertex: adjacency,
                            });
                        }
                        Entry::Occupied(mut entry) => {
                            if entry.get().1 > cost_to_adjacency {
                                entry.insert((vertex, cost_to_adjacency));
                                self.visitor.visit(&Event::EdgeRelaxed(edge), graph);
                                self.visitor.visit(&Event::DiscoverVertex(adjacency), graph);
                                self.fringe.push(State {
                                    evaluation: cost_to_adjacency + heuristic(&adjacency, graph),
                                    cost: cost_to_adjacency,
                                    vertex: adjacency,
                                });
                            } else {
                                self.visitor.visit(&Event::EdgeNotRelaxed(edge), graph);
                            }
                        }
                    }
                }
            }
            self.visitor.visit(&Event::FinishVertex(vertex), graph);
        }
        None
    }

    pub fn visitor_ref(&self) -> &V {
        &self.visitor
    }
}

#[cfg(test)]
mod tests {
    use super::{Astar, State};

    #[test]
    fn state() {
        use std::collections::BinaryHeap;
        use graph::{FromUsize, VertexDescriptor};

        let c1 = State {
            evaluation: 10,
            cost: 10,
            vertex: VertexDescriptor::from_usize(0),
        };
        let c2 = State {
            evaluation: 20,
            cost: 0,
            vertex: VertexDescriptor::from_usize(1),
        };
        let c3 = State {
            evaluation: 20,
            cost: 10,
            vertex: VertexDescriptor::from_usize(2),
        };
        let c4 = State {
            evaluation: 30,
            cost: 20,
            vertex: VertexDescriptor::from_usize(3),
        };

        assert!(c2 == c3);

        let mut cs = BinaryHeap::new();
        cs.push(c3.clone());
        cs.push(c2.clone());
        cs.push(c1.clone());
        cs.push(c4.clone());

        assert_eq!(cs.pop(), Some(c1));
        assert_eq!(cs.pop(), Some(c3));
        assert_eq!(cs.pop(), Some(c2));
        assert_eq!(cs.pop(), Some(c4));
    }

    #[test]
    fn astar() {
        use graph::{Directed, Graph, MutableGraph};
        use incidence_list::IncidenceList;

        let mut g = IncidenceList::<Directed, _, _>::new();

        let v0 = g.add_vertex(("s", 7));
        let v1 = g.add_vertex(("a", 6));
        let v2 = g.add_vertex(("b", 2));
        let v3 = g.add_vertex(("c", 1));
        let v4 = g.add_vertex(("g", 0));
        let v5 = g.add_vertex(("x", 0));

        g.add_edge(v0, v1, 1);
        g.add_edge(v0, v2, 4);
        g.add_edge(v1, v2, 2);
        g.add_edge(v1, v3, 5);
        g.add_edge(v1, v4, 12);
        g.add_edge(v2, v3, 2);
        g.add_edge(v3, v4, 3);

        assert_eq!(
            Astar::new().run(
                &v0,
                |&e, g| *g.edge_property(e).unwrap(),
                |&v, g| g.vertex_property(v).unwrap().1,
                |&v| v == v4,
                &g,
            ),
            Some(vec![v0, v1, v2, v3, v4])
        );
        assert_eq!(
            Astar::new().run(
                &v0,
                |&e, g| *g.edge_property(e).unwrap(),
                |&v, g| g.vertex_property(v).unwrap().1,
                |&v| v == v5,
                &g,
            ),
            None
        );
    }

    #[test]
    fn astar_with_visitor() {
        use graph::{Directed, Graph, MutableGraph, EdgeDescriptor, VertexDescriptor};
        use incidence_list::IncidenceList;
        use visitor::{Event, Visitor};

        struct MyVisitor {
            init: Vec<VertexDescriptor>,
            discovered: Vec<VertexDescriptor>,
            vertex_examined: Vec<VertexDescriptor>,
            edge_examined: Vec<EdgeDescriptor>,
            edge_relaxed: Vec<EdgeDescriptor>,
            edge_not_relaxed: Vec<EdgeDescriptor>,
            finished: Vec<VertexDescriptor>,
        }

        impl MyVisitor {
            fn new() -> Self {
                Self {
                    init: Vec::new(),
                    discovered: Vec::new(),
                    vertex_examined: Vec::new(),
                    edge_examined: Vec::new(),
                    edge_relaxed: Vec::new(),
                    edge_not_relaxed: Vec::new(),
                    finished: Vec::new(),
                }
            }
        }

        impl<'a, T> Visitor<T, Event> for MyVisitor
        where
            T: Graph,
        {
            fn visit(&mut self, e: &Event, _graph: &T) {
                match e {
                    &Event::InitializeVertex(v) => self.init.push(v),
                    &Event::DiscoverVertex(v) => self.discovered.push(v),
                    &Event::ExamineVertex(v) => self.vertex_examined.push(v),
                    &Event::ExamineEdge(e) => self.edge_examined.push(e),
                    &Event::EdgeRelaxed(e) => self.edge_relaxed.push(e),
                    &Event::EdgeNotRelaxed(e) => self.edge_not_relaxed.push(e),
                    &Event::FinishVertex(v) => self.finished.push(v),
                    _ => (),
                }
            }
        }

        let mut g = IncidenceList::<Directed, _, _>::new();

        let v0 = g.add_vertex(("s", 7));
        let v1 = g.add_vertex(("a", 6));
        let v2 = g.add_vertex(("b", 2));
        let v3 = g.add_vertex(("c", 1));
        let v4 = g.add_vertex(("g", 0));
        let _v5 = g.add_vertex(("x", 0));

        let e01 = g.add_edge(v0, v1, 1).unwrap();
        let e02 = g.add_edge(v0, v2, 4).unwrap();
        let e12 = g.add_edge(v1, v2, 2).unwrap();
        let e13 = g.add_edge(v1, v3, 5).unwrap();
        let e14 = g.add_edge(v1, v4, 12).unwrap();
        let e23 = g.add_edge(v2, v3, 2).unwrap();
        let e34 = g.add_edge(v3, v4, 3).unwrap();

        let mut astar = Astar::with_visitor(MyVisitor::new());

        assert_eq!(
            astar.run(
                &v0,
                |&e, g| *g.edge_property(e).unwrap(),
                |&v, g| g.vertex_property(v).unwrap().1,
                |&v| v == v4,
                &g,
            ),
            Some(vec![v0, v1, v2, v3, v4])
        );
        assert_eq!(astar.visitor_ref().init.len(), 6);
        assert_eq!(
            astar.visitor_ref().discovered,
            vec![v0, v1, v2, v3, v2, v4, v3, v4]
        );
        assert_eq!(
            astar.visitor_ref().vertex_examined,
            vec![v0, v2, v1, v2, v3, v3, v4]
        );
        assert_eq!(
            astar.visitor_ref().edge_examined,
            vec![e01, e02, e23, e12, e13, e14, e23, e34, e34]
        );
        assert_eq!(
            astar.visitor_ref().edge_relaxed,
            vec![e01, e02, e23, e12, e14, e23, e34]
        );
        assert_eq!(astar.visitor_ref().edge_not_relaxed, vec![e13, e34]);
        assert_eq!(astar.visitor_ref().finished, vec![v0, v2, v1, v2, v3, v3]);
    }
}
