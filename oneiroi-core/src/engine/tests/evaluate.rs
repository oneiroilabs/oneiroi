//! Evaluating instances: what runs, what is shared, what stays.

use super::*;
use crate::{key::ContentKey, types::DataType};

#[test]
fn evaluates_only_what_the_outputs_need() {
    let mut engine = Engine::default();
    let id = engine.add_program(program());
    let mut instance = engine.instantiate(id);

    assert_eq!(engine.evaluate([&mut instance]), 4);
    assert_eq!(
        engine.read::<f32>(&instance, "out"),
        Some(&[12.5, 24.5, 36.5][..])
    );
    assert_eq!(engine.read::<u32>(&instance, "count"), Some(&[3][..]));
    assert_eq!(engine.evaluate([&mut instance]), 0);
}

#[test]
fn equal_parameters_share_results() {
    let mut engine = Engine::default();
    let id = engine.add_program(program());
    let mut a = engine.instantiate(id);
    let mut b = engine.instantiate(id);
    let mut c = engine.instantiate(id);
    engine.set_param(&mut c, "offset", &[1.5f32]).unwrap();

    // `a` and `b` are the same work, `c` only differs behind `offset`.
    assert_eq!(engine.evaluate([&mut a, &mut b, &mut c]), 4 + 2);
    assert_eq!(engine.output_key(&a, "out"), engine.output_key(&b, "out"));
    assert_ne!(engine.output_key(&a, "out"), engine.output_key(&c, "out"));
    assert_eq!(
        engine.read::<f32>(&c, "out"),
        Some(&[13.5, 25.5, 37.5][..])
    );
}

#[test]
fn a_change_reruns_only_what_depends_on_it() {
    let mut engine = Engine::default();
    let id = engine.add_program(program());
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);

    engine.set_param(&mut instance, "scale", &[1f32]).unwrap();
    // scale and sum, but neither the doubling nor the offset.
    assert_eq!(engine.evaluate([&mut instance]), 2);
    assert_eq!(
        engine.read::<f32>(&instance, "out"),
        Some(&[3.5, 6.5, 9.5][..])
    );

    // Going back to a value that was seen before costs nothing.
    engine.set_param(&mut instance, "scale", &[10f32]).unwrap();
    assert_eq!(engine.evaluate([&mut instance]), 0);
}

#[test]
fn set_parameters_survive_eviction() {
    let mut engine = Engine::default();
    let id = engine.add_program(program());
    let mut instance = engine.instantiate(id);
    engine.set_param(&mut instance, "offset", &[1.5f32]).unwrap();
    engine.evaluate([&mut instance]);

    engine.end_frame(u64::MAX);
    engine.end_frame(0);
    assert_eq!(engine.evaluate([&mut instance]), 4);
    assert_eq!(
        engine.read::<f32>(&instance, "out"),
        Some(&[13.5, 25.5, 37.5][..])
    );

    // Once nothing holds it, the value may go like any other.
    let key = ContentKey::of_data(&f32::INFO, &[1.5f32.to_bits()]);
    drop(instance);
    engine.end_frame(u64::MAX);
    engine.end_frame(0);
    assert!(!engine.store().contains(key));
}

#[test]
fn evicted_results_are_recomputed() {
    let mut engine = Engine::default();
    let id = engine.add_program(program());
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);

    engine.end_frame(u64::MAX);
    engine.end_frame(0);
    // Only the constant `2` stays, which never changes while the
    // program is there.
    assert_eq!(engine.store().len(), 1);
    assert_eq!(engine.read::<f32>(&instance, "out"), None);

    assert_eq!(engine.evaluate([&mut instance]), 4);
    assert_eq!(
        engine.read::<f32>(&instance, "out"),
        Some(&[12.5, 24.5, 36.5][..])
    );
}

#[test]
fn an_instance_which_did_not_change_is_not_planned_and_keeps_its_outputs() {
    let mut engine = Engine::default();
    let id = engine.add_program(mostly_constant());
    let mut instance = engine.instantiate(id);
    let mut other = engine.instantiate(id);
    engine.set_param(&mut other, "scale", &[3.0f32]).unwrap();
    engine.evaluate([&mut instance, &mut other]);
    // Once all of its outputs are there it settles, and holds them.
    assert_eq!(engine.evaluate([&mut instance, &mut other]), 0);
    assert_eq!(instance.held.len(), 1);
    for _ in 0..3 {
        engine.end_frame(0);
    }
    // Nothing is planned for it, and nothing it made is evicted, frame
    // after frame.
    for _ in 0..3 {
        assert_eq!(engine.plan([&mut instance, &mut other]), 0);
        engine.flush();
        engine.end_frame(0);
    }
    assert_eq!(engine.read::<f32>(&instance, "moved"), Some(&[40.5, 60.5, 80.5][..]));

    // An edit lets go of what it held, which then goes like anything.
    engine.set_param(&mut instance, "scale", &[1.0f32]).unwrap();
    assert_eq!(engine.evaluate([&mut instance]), 2);
    assert!(instance.held.is_empty());
    assert_eq!(engine.read::<f32>(&instance, "moved"), Some(&[4.5, 6.5, 8.5][..]));
    engine.evaluate([&mut instance]);
    assert_eq!(instance.held.len(), 1);

    // Without the instances, nothing holds anything anymore.
    let edge = engine.statics[id.0 as usize].as_ref().unwrap().edge.len();
    drop((instance, other));
    engine.end_frame(0);
    engine.end_frame(0);
    assert_eq!(engine.store().len(), edge);
}

#[test]
fn a_program_can_call_another_one() {
    let mut engine = Engine::default();
    let inner = engine.add_program(program());

    let mut b = ProgramBuilder::new("outer");
    let values = b.param::<f32>("values", &[1., 2., 3.]);
    let three = b.constant::<f32>(&[3.]);
    let tripled = b.node(&SCALE, &[values, three]).unwrap()[0];
    // `scale` and `offset` keep the defaults of the inner program.
    let direct = b
        .call(engine.programs(), inner, &[Some(values), None, None])
        .unwrap();
    let nested = b
        .call(engine.programs(), inner, &[Some(tripled), None, None])
        .unwrap();
    b.output("direct", direct[0]);
    b.output("nested", nested[0]);
    let outer = engine.add_program(b.finish());

    let mut standalone = engine.instantiate(inner);
    assert_eq!(engine.evaluate([&mut standalone]), 4);

    // The direct call is the standalone instance again, so only the
    // tripling and the second call are new.
    let mut instance = engine.instantiate(outer);
    assert_eq!(engine.evaluate([&mut instance]), 1 + 4);
    assert_eq!(
        engine.output_key(&instance, "direct"),
        engine.output_key(&standalone, "out")
    );
    assert_eq!(
        engine.read::<f32>(&instance, "nested"),
        Some(&[36.5, 72.5, 108.5][..])
    );
}

#[test]
fn planning_and_running_can_be_apart() {
    let mut engine = Engine::default();
    let id = engine.add_program(program());
    let mut a = engine.instantiate(id);
    let mut b = engine.instantiate(id);
    engine.set_param(&mut b, "offset", &[1.5f32]).unwrap();

    // Planned in two goes, the second one only adds what is new.
    assert_eq!(engine.plan([&mut a]), 4);
    assert_eq!(engine.plan([&mut b]), 2);
    assert_eq!(engine.pending(), 6);

    // The keys are known, the values are not there yet.
    assert!(engine.output_key(&a, "out").is_some());
    assert_eq!(engine.read::<f32>(&a, "out"), None);

    assert_eq!(engine.flush(), 6);
    assert_eq!(engine.read::<f32>(&a, "out"), Some(&[12.5, 24.5, 36.5][..]));

    // What is not needed anymore can be dropped, and planned again.
    engine.set_param(&mut a, "scale", &[3f32]).unwrap();
    assert_eq!(engine.plan([&mut a]), 2);
    engine.discard();
    assert_eq!((engine.pending(), engine.flush()), (0, 0));
    assert_eq!(engine.plan([&mut a]), 2);
    engine.flush();
    assert_eq!(engine.read::<f32>(&b, "out"), Some(&[13.5, 25.5, 37.5][..]));
    assert_eq!((engine.pending(), engine.flush()), (0, 0));
}

#[test]
fn unused_programs_can_be_removed() {
    let mut engine = Engine::default();
    let inner = engine.add_program(program());
    let unused = engine.add_program(program());

    let mut b = ProgramBuilder::new("outer");
    let values = b.param::<f32>("values", &[1.]);
    let called = b
        .call(engine.programs(), inner, &[Some(values), None, None])
        .unwrap();
    b.output("out", called[0]);
    let outer = engine.add_program(b.finish());

    // `inner` stays because `outer` calls it.
    assert_eq!(engine.programs_mut().retain([outer]), 1);
    assert!(engine.programs().contains(inner) && !engine.programs().contains(unused));
    assert_eq!(engine.programs().len(), 2);

    let mut instance = engine.instantiate(outer);
    engine.evaluate([&mut instance]);
    assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[12.5][..]));

    // Ids are not reused, so an old one never names a new program.
    let next = engine.add_program(program());
    assert!(next != unused && engine.programs().find(program().hash).is_some());

    assert_eq!(engine.programs_mut().retain([]), 3);
    assert!(engine.programs().is_empty());
}

#[test]
fn types_are_checked() {
    let mut b = ProgramBuilder::new("broken");
    let count = b.constant::<u32>(&[1]);
    let values = b.param::<f32>("values", &[]);
    assert!(matches!(
        b.node(&SCALE, &[values, count]),
        Err(BuildError::Type { .. })
    ));
    assert!(matches!(
        b.node(&SCALE, &[values]),
        Err(BuildError::Arity { .. })
    ));

    let mut engine = Engine::default();
    let id = engine.add_program(program());
    let mut instance = engine.instantiate(id);
    assert!(matches!(
        engine.set_param(&mut instance, "scale", &[1u32]),
        Err(ParamError::Type { .. })
    ));
    assert_eq!(
        engine.set_param(&mut instance, "nope", &[1f32]),
        Err(ParamError::Unknown("nope".into()))
    );
}
