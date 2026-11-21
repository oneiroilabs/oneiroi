//! Everything people can see of the nodes is described, and nothing is
//! described which is not there.
#![cfg(feature = "meta")]

use std::collections::BTreeSet;

use oneiroi_core::node::{NodeRegistry, PortType};
use oneiroi_nodes::meta::MetaRegistry;

fn registries() -> (NodeRegistry, MetaRegistry) {
    let mut nodes = NodeRegistry::new();
    oneiroi_nodes::register(&mut nodes);
    let mut meta = MetaRegistry::new();
    oneiroi_nodes::describe(&mut meta);
    (nodes, meta)
}

/// Checks the ports of node `id` against what describes them.
fn check_ports(errors: &mut Vec<String>, id: &str, side: &str, names: &[&str], described: &[oneiroi_nodes::meta::PortMeta]) {
    for name in names {
        match described.iter().find(|port| port.name == *name) {
            None => errors.push(format!("{id}: {side} `{name}` is not described")),
            Some(port) if port.doc.trim().is_empty() => {
                errors.push(format!("{id}: {side} `{name}` has no documentation"))
            }
            Some(_) => {}
        }
    }
    for port in described {
        if !names.contains(&port.name) {
            errors.push(format!("{id}: {side} `{}` is described but not there", port.name));
        }
    }
}

#[test]
fn every_node_port_type_and_trait_is_described() {
    let (nodes, meta) = registries();
    let mut errors = Vec::new();
    let mut types = BTreeSet::new();
    let mut traits = BTreeSet::new();

    // The batches the engine runs where it knows there are many are not
    // seen by anybody: a graph has the single nodes.
    let batch = |id: &str| id.split('<').next().unwrap().ends_with(".batch");
    for node in nodes.iter().filter(|node| !batch(node.id)) {
        let Some(described) = meta.node(node.id) else {
            errors.push(format!("{}: not described", node.id));
            continue;
        };
        // A version of a generic node is described by it, with its ports.
        if described.id == node.id {
            let inputs: Vec<_> = node.inputs.iter().map(|port| port.name).collect();
            let outputs: Vec<_> = node.outputs.iter().map(|port| port.name).collect();
            check_ports(&mut errors, node.id, "input", &inputs, described.inputs);
            check_ports(&mut errors, node.id, "output", &outputs, described.outputs);
        }
        types.extend(node.inputs.iter().chain(node.outputs).map(|port| port.ty.name));
    }
    for generic in nodes.generics().filter(|generic| !batch(generic.id)) {
        let Some(described) = meta.node(generic.id) else {
            errors.push(format!("{}: not described", generic.id));
            continue;
        };
        let inputs: Vec<_> = generic.inputs.iter().map(|port| port.name).collect();
        let outputs: Vec<_> = generic.outputs.iter().map(|port| port.name).collect();
        check_ports(&mut errors, generic.id, "input", &inputs, described.inputs);
        check_ports(&mut errors, generic.id, "output", &outputs, described.outputs);
        for port in generic.inputs.iter().chain(generic.outputs) {
            match port.ty {
                PortType::Exact(ty) => types.insert(ty.name),
                PortType::Trait(tr) => traits.insert(tr.name),
            };
        }
    }
    types.extend(nodes.drawables().map(|drawable| drawable.ty.name));

    for described in meta.nodes() {
        if described.label.is_empty() || described.category.is_empty() || described.doc.trim().is_empty() {
            errors.push(format!("{}: needs a label, a category and documentation", described.id));
        }
    }
    for name in &types {
        match meta.ty(name) {
            None => errors.push(format!("type {name}: not described")),
            Some(ty) if ty.doc.trim().is_empty() || ty.label.is_empty() => {
                errors.push(format!("type {name}: needs a label and documentation"))
            }
            Some(_) => {}
        }
    }
    for name in &traits {
        match meta.tr(name) {
            None => errors.push(format!("trait {name}: not described")),
            Some(tr) if tr.doc.trim().is_empty() || tr.label.is_empty() => {
                errors.push(format!("trait {name}: needs a label and documentation"))
            }
            Some(_) => {}
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn documentation_reads_as_markdown() {
    let (_, meta) = registries();
    let sweep = meta.node("curve.sweep<circle>").expect("a version is described by its generic node");
    assert_eq!(sweep.label, "Sweep");
    let doc = oneiroi_nodes::meta::markdown(sweep.doc);
    assert!(doc.starts_with("The surface a profile leaves behind"), "{doc}");
    assert!(!doc.lines().any(|line| line.starts_with(' ')), "{doc}");
    assert_eq!(
        oneiroi_nodes::meta::summary(sweep.doc),
        "The surface a profile leaves behind when it is moved along a curve."
    );

    let nurbs = meta.node("curve.nurbs").unwrap();
    let points = nurbs.inputs.iter().find(|port| port.name == "points").unwrap();
    assert_eq!(points.label(), "Control points");
    let knots = nurbs.inputs.iter().find(|port| port.name == "knots").unwrap();
    assert_eq!(knots.label(), "Knots");
}
