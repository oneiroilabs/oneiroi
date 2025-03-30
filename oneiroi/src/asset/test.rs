use super::Asset;

#[test]
pub fn get_graph_instance() {
    let mut result = Asset::default();
    /* let node1 = result.add_node(
        Nodes::BoxV1(Box::new(BoxV1::default())),
        super::OneiroiNode {
            name: "Box".to_string(),
            postition: Vec2::new(0.0, 0.0),
        },
    );
    let node2 = result.add_node(
        Nodes::ExtrudeV1(Box::new(ExtrudeV1::default())),
        super::OneiroiNode {
            name: "Extrude".to_string(),
            postition: Vec2::new(0.0, 0.0),
        },
    );
    result.add_node_dependency(node1, 0, node2, 0); */
    let instance = result.get_instance();
}
