//! The constant part of a program, and folding it.

use super::*;
use crate::{engine::keys::Keys, program::{OpKind, SlotSource}};

#[test]
fn only_what_depends_on_a_parameter_derives_its_keys_again() {
    let mut engine = Engine::default();
    let id = engine.add_program(mostly_constant());
    let mut instance = engine.instantiate(id);
    let full = |engine: &Engine, instance: &Instance| {
        let program = engine.programs().get(id);
        Keys::derive(engine.programs(), program, &instance.param_keys).slots
    };
    engine.evaluate([&mut instance]);
    assert_eq!(instance.keys.as_ref().unwrap().slots, full(&engine, &instance));
    // Edit after edit, the keys are what deriving all of them gives.
    for (scale, offset) in [(3.0f32, None), (3.0, Some(1.0f32)), (4.0, Some(2.0))] {
        engine.set_param(&mut instance, "scale", &[scale]).unwrap();
        if let Some(offset) = offset {
            engine.set_param(&mut instance, "offset", &[offset]).unwrap();
        }
        engine.evaluate([&mut instance]);
        assert_eq!(instance.keys.as_ref().unwrap().slots, full(&engine, &instance));
    }
    assert_eq!(engine.read::<f32>(&instance, "moved"), Some(&[18.0, 26.0, 34.0][..]));
    // The parameter `offset` is read by the last node only.
    let statics = engine.statics[id.0 as usize].as_ref().unwrap();
    assert_eq!(statics.param_ops.iter().map(Vec::len).collect::<Vec<_>>(), [2, 1]);
    assert_eq!(statics.dynamic_ops, [2, 3]);
}

#[test]
fn the_edge_of_the_constant_part_stays_for_as_long_as_the_program() {
    let mut engine = Engine::default();
    let id = engine.add_program(mostly_constant());
    let mut instance = engine.instantiate(id);
    assert_eq!(engine.evaluate([&mut instance]), 4);
    // What the constant part ends in stays, what only it read goes:
    // `doubled` is an output and read by `shifted`, which is read by
    // what depends on `scale`. The values and the `2` are not needed
    // anymore once they ran. What was used in a frame stays for it.
    engine.end_frame(0);
    engine.end_frame(0);
    let edge = engine.statics[id.0 as usize].as_ref().unwrap().edge.len();
    assert_eq!(edge, 2);
    assert_eq!(engine.store().len(), edge);
    assert_eq!(engine.read::<f32>(&instance, "constant"), Some(&[2.0, 4.0, 6.0][..]));

    // An edit runs what depends on it, and nothing of the rest, even
    // with everything else evicted every frame.
    for scale in [3.0f32, 4.0, 5.0] {
        engine.set_param(&mut instance, "scale", &[scale]).unwrap();
        assert_eq!(engine.evaluate([&mut instance]), 2);
        engine.end_frame(0);
    }

    // Without the program and its instance, nothing keeps it anymore.
    drop(instance);
    engine.programs_mut().retain([]);
    engine.end_frame(0);
    engine.end_frame(0);
    assert!(engine.store().is_empty());
}

#[test]
fn a_folded_program_runs_only_what_depends_on_a_parameter() {
    let mut engine = Engine::default();
    let id = engine.add_program(mostly_constant());
    let folded = engine.fold(id);
    let program = engine.programs().get(folded);
    assert_eq!(program.ops.len(), 2);
    assert_eq!(program.params.len(), 2);
    // The constant output is a constant now, of what it was computed to.
    let constant = program.slot(program.outputs[program.output_index("constant").unwrap()].slot);
    assert!(matches!(&constant.source, SlotSource::Const { words, .. } if words.len() == 3));

    // The same results under the same keys, edit after edit.
    let (mut a, mut b) = (engine.instantiate(id), engine.instantiate(folded));
    for (scale, offset) in [(10.0f32, 0.5f32), (3.0, 0.5), (3.0, 1.0)] {
        for instance in [&mut a, &mut b] {
            engine.set_param(instance, "scale", &[scale]).unwrap();
            engine.set_param(instance, "offset", &[offset]).unwrap();
        }
        engine.evaluate([&mut a]);
        // Everything the folded one needs was made for the other one.
        assert_eq!(engine.evaluate([&mut b]), 0);
        for output in ["moved", "constant"] {
            assert_eq!(engine.output_key(&a, output), engine.output_key(&b, output));
            assert_eq!(engine.read::<f32>(&a, output), engine.read::<f32>(&b, output));
        }
    }
}

#[test]
fn a_call_with_only_constants_folds_into_its_result() {
    let mut engine = Engine::default();
    let inner = engine.add_program(program());
    let mut b = ProgramBuilder::new("outer");
    let values = b.constant::<f32>(&[1., 2., 3.]);
    let called = b.call(engine.programs(), inner, &[Some(values), None, None]).unwrap();
    let by = b.param::<f32>("by", &[2.]);
    let scaled = b.node(&SCALE, &[called[0], by]).unwrap()[0];
    b.output("scaled", scaled);
    let outer = engine.add_program(b.finish());

    let folded = engine.fold(outer);
    let program = engine.programs().get(folded);
    assert_eq!(program.ops.len(), 1);
    assert!(matches!(program.ops[0].kind, OpKind::Node(_)));
    let mut instance = engine.instantiate(folded);
    engine.set_param(&mut instance, "by", &[0.5f32]).unwrap();
    engine.evaluate([&mut instance]);
    assert_eq!(engine.read::<f32>(&instance, "scaled"), Some(&[6.25, 12.25, 18.25][..]));
}
