//! Prints out all HTML elements from a page that were modified at least 4 times, then prints out
//! those nodes and their respective modifications sorted by timestamp.

use pagegraph::from_xml::read_from_file;
use pagegraph::types::NodeType;

fn main() {
    let graph_file = std::env::args()
        .skip(1)
        .next()
        .expect("Provide a path to a `.graphml` file");
    let graph = read_from_file(&graph_file);

    let html_elements = graph.filter_nodes(|node_type| match node_type {
        NodeType::HtmlElement { .. } => true,
        _ => false,
    });

    let mut heavily_modified_elements: Vec<_> = html_elements
        .iter()
        .filter_map(|(node_id, _node)| {
            let num_modifications = graph.all_html_element_modifications(**node_id).len();
            if num_modifications >= 4 {
                Some((*node_id, num_modifications))
            } else {
                None
            }
        })
        .collect();

    heavily_modified_elements.sort_by(|(_, a), (_, b)| b.cmp(a));

    heavily_modified_elements
        .iter()
        .rev()
        .for_each(|(id, num)| {
            println!("{:?} was modified {} times", id, num);
        });

    heavily_modified_elements
        .iter()
        .map(|(id, _)| *id)
        .for_each(|id| {
            let modifications = graph.all_html_element_modifications(*id);
            dbg!(graph.nodes.get(id));
            dbg!(modifications);
        });
}
