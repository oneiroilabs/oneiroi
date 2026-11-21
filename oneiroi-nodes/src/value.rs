//! Values as nodes, so a list typed in once can be linked to many places.

use glam::Vec3;
use oneiroi_core::{
    node::{Caps, GpuKernel, GpuOutput, Inputs, NodeRegistry, Outputs, PortDecl},
    types::DataType,
};

fn pass<T: DataType>(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    outputs.set(0, inputs.get::<T>(0));
}

node! {
    /// A list of numbers, typed in once and linked to as many places as need it.
    pub static F32 = "value.f32" {
        label: "Numbers",
        category: "Value",
        version: 1,
        inputs {
            /// The numbers.
            port PortDecl::new::<f32>("values").with_default(&[0.0f32]);
        }
        outputs {
            /// The same numbers.
            port PortDecl::new::<f32>("values").follows(&[0]);
        }
        cpu: pass::<f32>,
        gpu: &[],
    }
}

node! {
    /// A list of points or directions, typed in once and linked to as many
    /// places as need them.
    pub static VEC3 = "value.vec3" {
        label: "Vectors",
        category: "Value",
        version: 1,
        inputs {
            /// The vectors.
            port PortDecl::new::<Vec3>("values").with_default(&[Vec3::ZERO]);
        }
        outputs {
            /// The same vectors.
            port PortDecl::new::<Vec3>("values").follows(&[0]);
        }
        cpu: pass::<Vec3>,
        gpu: &[],
    }
}

/// How many elements a node makes of lists which are paired up by their
/// index, the shorter repeating their last element: none if one is empty.
fn paired(lens: &[usize]) -> usize {
    match lens.contains(&0) {
        true => 0,
        false => lens.iter().copied().max().unwrap_or(0),
    }
}

/// Element `index` of `values`, or its last one past its end.
fn at<T: Copy>(values: &[T], index: usize) -> T {
    values[index.min(values.len() - 1)]
}

const OPERATIONS: &[&str] = &["add", "subtract", "multiply", "divide", "minimum", "maximum"];

node! {
    /// Numbers worked out of two lists of them, element by element: `a` plus,
    /// minus, times or divided by `b`, or the smaller or the larger of both.
    /// The shorter list repeats its last number.
    pub static MATH = "value.math" {
        label: "Math",
        category: "Value",
        version: 1,
        inputs {
            /// The numbers on the left.
            port PortDecl::new::<f32>("a").with_default(&[0.0f32]);
            /// The numbers on the right.
            port PortDecl::new::<f32>("b").with_default(&[0.0f32]);
            /// What is done with each pair.
            port PortDecl::new::<u32>("operation").choices(OPERATIONS);
        }
        outputs {
            /// What came out, as many as the longer list has: a single number of
            /// two single ones.
            port PortDecl::new::<f32>("values").follows(&[0, 1]);
        }
        cpu: math,
        gpu: &[GpuKernel {
            wgsl: "
fn math(index: u32) -> f32 {
    let a = in_a(min(index, in_a_len() - 1u));
    let b = in_b(min(index, in_b_len() - 1u));
    switch (min(in_operation(0u), 5u)) {
        case 0u: { return a + b; }
        case 1u: { return a - b; }
        case 2u: { return a * b; }
        case 3u: { return a / b; }
        case 4u: { return min(a, b); }
        default: { return max(a, b); }
    }
}
",
            outputs: &[GpuOutput {
                entry: "math",
                len: |inputs| paired(&[inputs.len(0), inputs.len(1)]),
                words: None,
            }],
            requires: Caps::NONE,
            reads: &[],
            accepts: None,
            examples: &[
                || vec![words(&[1.0f32, 2.0, 3.0]), words(&[0.5f32]), vec![0]],
                || vec![words(&[1.0f32, -2.0]), words(&[4.0f32, 8.0, 16.0]), vec![1]],
                || vec![words(&[1.5f32]), words(&[2.0f32, -3.0]), vec![2]],
                || vec![words(&[3.0f32, 9.0]), words(&[2.0f32, 4.0]), vec![3]],
                || vec![words(&[3.0f32, -9.0]), words(&[2.0f32]), vec![4]],
                || vec![words(&[3.0f32, -9.0]), words(&[2.0f32]), vec![5]],
                || vec![Vec::new(), words(&[2.0f32]), vec![0]],
            ],
            stages: &[],
        }],
    }
}

fn math(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let (a, b) = (inputs.get::<f32>(0), inputs.get::<f32>(1));
    let operation: fn(f32, f32) -> f32 = match inputs.choice(2) {
        0 => |a, b| a + b,
        1 => |a, b| a - b,
        2 => |a, b| a * b,
        3 => |a, b| a / b,
        4 => f32::min,
        _ => f32::max,
    };
    let values: Vec<f32> = (0..paired(&[a.len(), b.len()])).map(|index| operation(at(a, index), at(b, index))).collect();
    outputs.set(0, &values);
}

node! {
    /// Vectors made of their `x`, `y` and `z`, element by element. A shorter list
    /// repeats its last number.
    pub static COMBINE_XYZ = "value.combine_xyz" {
        label: "Combine XYZ",
        category: "Value",
        version: 1,
        inputs {
            /// How far along `x`.
            port PortDecl::new::<f32>("x").with_default(&[0.0f32]);
            /// How far along `y`, which is up.
            port PortDecl::new::<f32>("y").with_default(&[0.0f32]);
            /// How far along `z`.
            port PortDecl::new::<f32>("z").with_default(&[0.0f32]);
        }
        outputs {
            /// The vectors, as many as the longest list has: a single vector of
            /// single numbers.
            port PortDecl::new::<Vec3>("vectors").follows(&[0, 1, 2]);
        }
        cpu: combine_xyz,
        gpu: &[GpuKernel {
            wgsl: "
fn combine(index: u32) -> vec3<f32> {
    return vec3<f32>(
        in_x(min(index, in_x_len() - 1u)),
        in_y(min(index, in_y_len() - 1u)),
        in_z(min(index, in_z_len() - 1u)),
    );
}
",
            outputs: &[GpuOutput {
                entry: "combine",
                len: |inputs| paired(&[inputs.len(0), inputs.len(1), inputs.len(2)]),
                words: None,
            }],
            requires: Caps::NONE,
            reads: &[],
            accepts: None,
            examples: &[
                || vec![words(&[1.0f32, 2.0, 3.0]), words(&[0.5f32]), words(&[-1.0f32, -2.0])],
                || vec![words(&[1.0f32]), Vec::new(), words(&[2.0f32])],
            ],
            stages: &[],
        }],
    }
}

fn combine_xyz(inputs: &Inputs<'_>, outputs: &mut Outputs) {
    let (x, y, z) = (inputs.get::<f32>(0), inputs.get::<f32>(1), inputs.get::<f32>(2));
    let vectors: Vec<Vec3> = (0..paired(&[x.len(), y.len(), z.len()]))
        .map(|index| Vec3::new(at(x, index), at(y, index), at(z, index)))
        .collect();
    outputs.set(0, &vectors);
}

fn words<T: bytemuck::Pod>(values: &[T]) -> Vec<u32> {
    bytemuck::cast_slice(values).to_vec()
}

describe_type! {
    /// A number with a fraction, like `0.5` or `-3`.
    f32 { label: "Number", hue: Sky }
}

describe_type! {
    /// A whole number, never below `0`, like a count or an index.
    u32 { label: "Whole number", hue: Cyan }
}

describe_type! {
    /// Two numbers `x` and `y`: a place on a surface or in a plane.
    glam::Vec2 { label: "2D vector", hue: Indigo }
}

describe_type! {
    /// Three numbers `x`, `y` and `z`: a point or a direction in space.
    Vec3 { label: "Vector", hue: Violet }
}

pub fn register(registry: &mut NodeRegistry) {
    registry.register(&F32);
    registry.register(&VEC3);
    registry.register(&MATH);
    registry.register(&COMBINE_XYZ);
}

#[cfg(test)]
mod tests {
    use oneiroi_core::{engine::Engine, program::ProgramBuilder};

    use super::*;

    fn math_of(a: &[f32], b: &[f32], operation: u32) -> Vec<f32> {
        let mut b_ = ProgramBuilder::new("math");
        let (a, b, operation) = (b_.constant(a), b_.constant(b), b_.constant(&[operation]));
        let values = b_.node(&MATH, &[a, b, operation]).unwrap()[0];
        b_.output("values", values);
        let mut engine = Engine::default();
        let id = engine.add_program(b_.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);
        engine.read::<f32>(&instance, "values").unwrap().to_vec()
    }

    #[test]
    fn math_works_element_by_element() {
        assert_eq!(math_of(&[1.0, 2.0, 3.0], &[10.0], 0), [11.0, 12.0, 13.0]);
        assert_eq!(math_of(&[1.0], &[10.0, 20.0], 1), [-9.0, -19.0]);
        assert_eq!(math_of(&[2.0, 3.0], &[4.0, 5.0], 2), [8.0, 15.0]);
        assert_eq!(math_of(&[9.0], &[3.0], 3), [3.0]);
        assert_eq!(math_of(&[1.0, 5.0], &[3.0], 4), [1.0, 3.0]);
        assert_eq!(math_of(&[1.0, 5.0], &[3.0], 5), [3.0, 5.0]);
        // Past the last operation is the last one, and nothing gives nothing.
        assert_eq!(math_of(&[1.0, 5.0], &[3.0], 99), [3.0, 5.0]);
        assert!(math_of(&[], &[3.0], 0).is_empty());
    }

    #[test]
    fn vectors_are_made_of_their_parts() {
        let mut b = ProgramBuilder::new("xyz");
        let (x, y, z) = (b.constant(&[1.0f32, 2.0]), b.constant(&[5.0f32]), b.constant(&[-1.0f32, -2.0, -3.0]));
        let vectors = b.node(&COMBINE_XYZ, &[x, y, z]).unwrap()[0];
        b.output("vectors", vectors);
        let mut engine = Engine::default();
        let id = engine.add_program(b.finish());
        let mut instance = engine.instantiate(id);
        engine.evaluate([&mut instance]);
        assert_eq!(
            engine.read::<Vec3>(&instance, "vectors").unwrap(),
            [Vec3::new(1.0, 5.0, -1.0), Vec3::new(2.0, 5.0, -2.0), Vec3::new(2.0, 5.0, -3.0)]
        );
    }
}
