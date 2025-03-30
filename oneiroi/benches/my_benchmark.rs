use criterion::{Criterion, criterion_group, criterion_main};
use oneiroi::{
    asset::Asset,
    data_types::Property,
    mesh::OneiroiMesh,
    operations::{Operation, PropertyInterface, producers::cylinder::CylinderV1},
};

use std::time::Duration;

fn construct_box_extrude() -> OneiroiMesh {
    /* let ps_box = BoxV1::default().compute(());

    let new_mesh = ExtrudeV1::default().compute(ps_box);

    new_mesh */
    //TODO
    OneiroiMesh::default()
}

fn construct_extrude_graph() -> Asset {
    let mut result = Asset::default();
    //TODO if sure
    /* let node1 = result.add_node(Nodes::BoxV1(BoxV1::default()));
    let node2 = result.add_node(Nodes::ExtrudeV1(ExtrudeV1::default()));
    result.add_node_dependency(node1, 0, node2, 0); */
    result
}

fn compute_cylinder_500() {
    let mut cyl = CylinderV1::default();
    _ = cyl.try_set_property(
        "segments",
        oneiroi::data_types::DataTypeInstance::Int(Property::new(500)),
    );
    cyl.compute(vec![]);
}

fn criterion_benchmark(c: &mut Criterion) {
    //c.bench_function(" Construct", |b| b.iter(|| BoxV1::default().compute(())));
    //c.bench_function(" Construct  and Extrude", |b| b.iter(construct_box_extrude));
    /* c.bench_function("Construct Extrude Graph", |b| {
        b.iter(construct_extrude_graph)
    }); */
    c.bench_function("Compute_Cylinder_500Sub", |b| b.iter(compute_cylinder_500));
}

/* fn criterion_benchmark2(c: &mut Criterion) {
    c.bench_function(" Construct  and Extrude", |b| {
        b.iter(|| construct_box_extrude(/* black_box(20) */))
    });
} */

criterion_group! {
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().significance_level(0.1).sample_size(500).warm_up_time(Duration::from_secs(5)).measurement_time(Duration::from_secs(10));
    targets = criterion_benchmark
}
//criterion_group!(benches2, criterion_benchmark2);
criterion_main!(benches);
