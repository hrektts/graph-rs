use fnv::FnvHashMap;

use graph::VertexDescriptor;

pub fn reverse_path(
    parents: &FnvHashMap<VertexDescriptor, VertexDescriptor>,
    goal: VertexDescriptor,
) -> Vec<VertexDescriptor> {
    let mut path = vec![goal];
    while let Some(parent) = parents.get(path.last().unwrap()) {
        path.push(*parent);
    }
    path.reverse();
    path
}
