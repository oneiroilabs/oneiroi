mod constant;
mod evaluate;
mod for_each;
mod generic;
mod groups;
mod instances;
mod link;

use glam::Vec3;

use super::*;
use crate::{node::{Inputs, NodeDef, Outputs, PortDecl}, program::{BuildError, ProgramBuilder}};

fn zip(inputs: &Inputs<'_>, outputs: &mut Outputs, f: fn(f32, f32) -> f32) {
    let (a, b) = (inputs.get::<f32>(0), inputs.value::<f32>(1));
    let out = outputs.alloc::<f32>(0, a.len());
    for (out, a) in out.iter_mut().zip(a) {
        *out = f(*a, b);
    }
}

const BINARY_IN: &[PortDecl] = &[PortDecl::new::<f32>("values"), PortDecl::new::<f32>("by")];

const BINARY_OUT: &[PortDecl] = &[PortDecl::new::<f32>("values")];

const fn binary(id: &'static str, cpu: crate::node::CpuKernel) -> NodeDef {
    NodeDef {
        id,
        inputs: BINARY_IN,
        outputs: BINARY_OUT,
        version: 0,
        cpu,
        gpu: &[],
    }
}

static SCALE: NodeDef = binary("test.scale", |i, o| zip(i, o, |a, b| a * b));

static OFFSET: NodeDef = binary("test.offset", |i, o| zip(i, o, |a, b| a + b));

static SUM: NodeDef = NodeDef {
    id: "test.sum",
    inputs: &[PortDecl::new::<f32>("a"), PortDecl::new::<f32>("b")],
    outputs: &[PortDecl::new::<f32>("sum"), PortDecl::new::<u32>("count")],
    version: 0,
    cpu: |inputs, outputs| {
        let (a, b) = (inputs.get::<f32>(0), inputs.get::<f32>(1));
        let sum: Vec<_> = a.iter().zip(b).map(|(a, b)| a + b).collect();
        outputs.set(0, &sum);
        outputs.set(1, &[sum.len() as u32]);
    },
    gpu: &[],
};

/// out = values * scale + values * 2 + offset
fn program() -> crate::program::Program {
    let mut b = ProgramBuilder::new("test");
    let values = b.param::<f32>("values", &[1., 2., 3.]);
    let scale = b.param::<f32>("scale", &[10.]);
    let offset = b.param::<f32>("offset", &[0.5]);
    let two = b.constant::<f32>(&[2.]);

    let scaled = b.node(&SCALE, &[values, scale]).unwrap()[0];
    let doubled = b.node(&SCALE, &[values, two]).unwrap()[0];
    let shifted = b.node(&OFFSET, &[doubled, offset]).unwrap()[0];
    let sum = b.node(&SUM, &[scaled, shifted]).unwrap();
    // Nothing reads this, so it must never run.
    b.node(&OFFSET, &[scaled, offset]).unwrap();

    b.output("out", sum[0]);
    b.output("count", sum[1]);
    b.finish()
}

/// Constants into a chain, the end of which a parameter changes, and a
/// constant output of their own.
fn mostly_constant() -> crate::program::Program {
    let mut b = ProgramBuilder::new("mostly constant");
    let values = b.constant::<f32>(&[1., 2., 3.]);
    let two = b.constant::<f32>(&[2.]);
    let doubled = b.node(&SCALE, &[values, two]).unwrap()[0];
    let shifted = b.node(&OFFSET, &[doubled, two]).unwrap()[0];
    let scale = b.param::<f32>("scale", &[10.]);
    let offset = b.param::<f32>("offset", &[0.5]);
    let scaled = b.node(&SCALE, &[shifted, scale]).unwrap()[0];
    let moved = b.node(&OFFSET, &[scaled, offset]).unwrap()[0];
    b.output("moved", moved);
    b.output("constant", doubled);
    b.finish()
}
