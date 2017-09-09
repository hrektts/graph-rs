use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::marker::PhantomData;

use fnv::FnvHashMap;

use graph::{Graph, AdjacencyGraph, AdjacencyMatrixGraph, VertexListGraph, VertexDescriptor};
use path::reverse_path;
use visitor::{Event, Visitor, DefaultVisitor};

pub struct Bfs<G, V>
where
    G: Graph,
    V: Visitor<G, Event>,
{
    fringe: VecDeque<VertexDescriptor>,
    parents: FnvHashMap<VertexDescriptor, VertexDescriptor>,
    visitor: V,
    phantom: PhantomData<G>,
}

impl<G> Bfs<G, DefaultVisitor>
where
    G: Graph,
{
    pub fn new() -> Self {
        Self {
            fringe: VecDeque::new(),
            parents: FnvHashMap::default(),
            visitor: DefaultVisitor,
            phantom: PhantomData,
        }
    }
}

impl<G, V> Bfs<G, V>
where
    G: Graph,
    V: Visitor<G, Event>,
{
    pub fn with_visitor(visitor: V) -> Self {
        Self {
            fringe: VecDeque::new(),
            parents: FnvHashMap::default(),
            visitor: visitor,
            phantom: PhantomData,
        }
    }

    pub fn run<'a, F>(
        &mut self,
        start: &VertexDescriptor,
        is_goal: F,
        graph: &'a G,
    ) -> Option<Vec<VertexDescriptor>>
    where
        F: Fn(&VertexDescriptor) -> bool,
        G: AdjacencyGraph<'a> + AdjacencyMatrixGraph + VertexListGraph<'a>,
    {
        for vertex in graph.vertices() {
            self.visitor.visit(&Event::InitializeVertex(vertex), graph)
        }

        self.visitor.visit(&Event::DiscoverVertex(*start), graph);
        self.fringe.push_back(*start);

        while let Some(vertex) = self.fringe.pop_front() {
            self.visitor.visit(&Event::ExamineVertex(vertex), graph);
            if is_goal(&vertex) {
                return Some(reverse_path(&self.parents, vertex));
            }
            for adjacency in graph.adjacent_vertices(vertex) {
                let edge = graph.edge(vertex, adjacency).unwrap();
                self.visitor.visit(&Event::ExamineEdge(edge), graph);
                if adjacency != *start {
                    if let Entry::Vacant(entry) = self.parents.entry(adjacency) {
                        self.visitor.visit(&Event::TreeEdge(edge), graph);
                        entry.insert(vertex);
                        self.visitor.visit(&Event::DiscoverVertex(adjacency), graph);
                        self.fringe.push_back(adjacency);
                    } else {
                        self.visitor.visit(&Event::NonTreeEdge(edge), graph);
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
    use super::Bfs;

    #[test]
    fn bfs() {
        use graph::{Directed, MutableGraph};
        use incidence_list::IncidenceList;

        let mut g = IncidenceList::<Directed, _, _>::new();

        let v0 = g.add_vertex("a");
        let v1 = g.add_vertex("b");
        let v2 = g.add_vertex("c");
        let v3 = g.add_vertex("d");
        let v4 = g.add_vertex("e");
        let v5 = g.add_vertex("f");
        let v6 = g.add_vertex("g");
        let v7 = g.add_vertex("h");
        let v8 = g.add_vertex("i");
        let v9 = g.add_vertex("j");

        g.add_edge(v0, v1, ());
        g.add_edge(v0, v4, ());
        g.add_edge(v1, v5, ());
        g.add_edge(v2, v0, ());
        g.add_edge(v2, v4, ());
        g.add_edge(v4, v1, ());
        g.add_edge(v4, v3, ());
        g.add_edge(v4, v6, ());
        g.add_edge(v5, v4, ());
        g.add_edge(v5, v4, ());
        g.add_edge(v6, v7, ());
        g.add_edge(v7, v3, ());
        g.add_edge(v7, v9, ());
        g.add_edge(v8, v7, ());
        g.add_edge(v9, v8, ());

        assert_eq!(
            Bfs::new().run(&v0, |&v| v == v9, &g),
            Some(vec![v0, v4, v6, v7, v9])
        );
        assert_eq!(Bfs::new().run(&v0, |&v| v == v2, &g), None);
    }

    #[test]
    fn bfs_with_visitor() {
        use graph::{Graph, Directed, IncidenceGraph, MutableGraph, VertexDescriptor};
        use incidence_list::IncidenceList;
        use visitor::{Event, Visitor};

        struct MyVisitor {
            init: Vec<VertexDescriptor>,
            discovered: Vec<VertexDescriptor>,
            vertex_examined: Vec<VertexDescriptor>,
            edge_target_examined: Vec<VertexDescriptor>,
            tree_edge_target: Vec<VertexDescriptor>,
            non_tree_edge_target: Vec<VertexDescriptor>,
            finished: Vec<VertexDescriptor>,
        }

        impl MyVisitor {
            fn new() -> Self {
                Self {
                    init: Vec::new(),
                    discovered: Vec::new(),
                    vertex_examined: Vec::new(),
                    edge_target_examined: Vec::new(),
                    tree_edge_target: Vec::new(),
                    non_tree_edge_target: Vec::new(),
                    finished: Vec::new(),
                }
            }
        }

        impl<'a, G> Visitor<G, Event> for MyVisitor
        where
            G: Graph + IncidenceGraph<'a>,
        {
            fn visit(&mut self, e: &Event, graph: &G) {
                match e {
                    &Event::InitializeVertex(v) => self.init.push(v),
                    &Event::DiscoverVertex(v) => self.discovered.push(v),
                    &Event::ExamineVertex(v) => self.vertex_examined.push(v),
                    &Event::ExamineEdge(e) => self.edge_target_examined.push(graph.target(e)),
                    &Event::TreeEdge(e) => self.tree_edge_target.push(graph.target(e)),
                    &Event::NonTreeEdge(e) => self.non_tree_edge_target.push(graph.target(e)),
                    &Event::FinishVertex(v) => self.finished.push(v),
                    _ => (),
                }
            }
        }

        let mut g = IncidenceList::<Directed, _, _>::new();

        let v0 = g.add_vertex("a");
        let v1 = g.add_vertex("b");
        let v2 = g.add_vertex("c");
        let v3 = g.add_vertex("d");
        let v4 = g.add_vertex("e");
        let v5 = g.add_vertex("f");
        let v6 = g.add_vertex("g");
        let v7 = g.add_vertex("h");
        let v8 = g.add_vertex("i");
        let v9 = g.add_vertex("j");

        g.add_edge(v0, v1, ());
        g.add_edge(v0, v4, ());
        g.add_edge(v1, v5, ());
        g.add_edge(v2, v0, ());
        g.add_edge(v2, v4, ());
        g.add_edge(v4, v1, ());
        g.add_edge(v4, v3, ());
        g.add_edge(v4, v6, ());
        g.add_edge(v5, v4, ());
        g.add_edge(v5, v4, ());
        g.add_edge(v6, v7, ());
        g.add_edge(v7, v3, ());
        g.add_edge(v7, v9, ());
        g.add_edge(v8, v7, ());
        g.add_edge(v9, v8, ());

        let mut bfs = Bfs::with_visitor(MyVisitor::new());

        assert_eq!(
            bfs.run(&v0, |&v| v == v9, &g),
            Some(vec![v0, v4, v6, v7, v9])
        );
        assert_eq!(bfs.visitor_ref().init.len(), 10);
        assert_eq!(
            bfs.visitor_ref().discovered,
            vec![v0, v1, v4, v5, v3, v6, v7, v9]
        );
        assert_eq!(
            bfs.visitor_ref().vertex_examined,
            vec![v0, v1, v4, v5, v3, v6, v7, v9]
        );
        assert_eq!(
            bfs.visitor_ref().edge_target_examined,
            vec![v1, v4, v5, v1, v3, v6, v4, v7, v3, v9]
        );
        assert_eq!(
            bfs.visitor_ref().tree_edge_target,
            vec![v1, v4, v5, v3, v6, v7, v9]
        );
        assert_eq!(bfs.visitor_ref().non_tree_edge_target, vec![v1, v4, v3]);
        assert_eq!(bfs.visitor_ref().finished, vec![v0, v1, v4, v5, v3, v6, v7]);
    }
}
