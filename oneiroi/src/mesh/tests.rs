use insta::assert_yaml_snapshot;

use crate::operations::{
    Operation,
    producers::{r#box::BoxV1, cylinder::CylinderV1},
};

#[test]
fn compute_cylinder() {
    assert_yaml_snapshot!(CylinderV1::default().compute(vec![]))
}

#[test]
fn compute_box() {
    assert_yaml_snapshot!(BoxV1::default().compute(vec![]))
}
