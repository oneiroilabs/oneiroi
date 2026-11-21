//! Parameters which follow outputs of other instances.

use super::*;

/// What instances follow in the tests of links: `1, 2, 3` times
/// `scale`.
fn scaled() -> crate::program::Program {
    let mut b = ProgramBuilder::new("scaled");
    let values = b.constant::<f32>(&[1., 2., 3.]);
    let scale = b.param::<f32>("scale", &[1.]);
    let scaled = b.node(&SCALE, &[values, scale]).unwrap()[0];
    b.output("scaled", scaled);
    b.finish()
}

/// What follows: `values` plus `offset`.
fn moved() -> crate::program::Program {
    let mut b = ProgramBuilder::new("moved");
    let values = b.param::<f32>("values", &[]);
    let offset = b.param::<f32>("offset", &[0.5]);
    let moved = b.node(&OFFSET, &[values, offset]).unwrap()[0];
    b.output("moved", moved);
    b.finish()
}

#[test]
fn a_parameter_follows_an_output_of_another_instance_in_the_same_frame() {
    let mut engine = Engine::default();
    let (scaled, moved) = (engine.add_program(scaled()), engine.add_program(moved()));
    let (mut source, mut follower) = (engine.instantiate(scaled), engine.instantiate(moved));
    engine.link(&mut follower, "values", &source, "scaled").unwrap();
    // The one which follows comes first, and still sees what it follows
    // as it is now.
    assert_eq!(engine.evaluate([&mut follower, &mut source]), 2);
    assert_eq!(engine.read::<f32>(&follower, "moved"), Some(&[1.5, 2.5, 3.5][..]));

    for scale in [2.0f32, 3.0] {
        engine.set_param(&mut source, "scale", &[scale]).unwrap();
        assert_eq!(engine.evaluate([&mut follower, &mut source]), 2);
        let expected: Vec<f32> = [1.0, 2.0, 3.0].iter().map(|value| value * scale + 0.5).collect();
        assert_eq!(engine.read::<f32>(&follower, "moved"), Some(&expected[..]));
    }
    // Nothing is copied: the parameter is the output.
    let index = engine.programs().get(moved).param_index("values").unwrap();
    assert_eq!(Some(follower.param_keys[index]), engine.output_key(&source, "scaled"));
    // Neither changed, so nothing runs.
    assert_eq!(engine.evaluate([&mut follower, &mut source]), 0);
}

#[test]
fn what_is_followed_is_made_again_even_without_its_instance() {
    let mut engine = Engine::default();
    let (scaled, moved) = (engine.add_program(scaled()), engine.add_program(moved()));
    let (mut source, mut follower) = (engine.instantiate(scaled), engine.instantiate(moved));
    engine.link(&mut follower, "values", &source, "scaled").unwrap();
    engine.set_param(&mut source, "scale", &[4.0f32]).unwrap();
    engine.evaluate([&mut source]);
    // Evicted before the follower ever saw it, and its instance gone.
    drop(source);
    engine.end_frame(0);
    engine.end_frame(0);
    assert_eq!(engine.evaluate([&mut follower]), 2);
    assert_eq!(engine.read::<f32>(&follower, "moved"), Some(&[4.5, 8.5, 12.5][..]));

    // Setting the parameter ends it, and once nobody follows the source
    // nothing of it is held anymore.
    engine.set_param(&mut follower, "values", &[1.0f32]).unwrap();
    assert_eq!(engine.evaluate([&mut follower]), 1);
    assert_eq!(engine.read::<f32>(&follower, "moved"), Some(&[1.5][..]));
    drop(follower);
    engine.end_frame(0);
    engine.end_frame(0);
    engine.end_frame(0);
    // Only the edges of the constant parts are left.
    let edges: usize = engine.statics.iter().flatten().map(|statics| statics.edge.len()).sum();
    assert_eq!(engine.store().len(), edges);
}

#[test]
fn links_never_go_round_in_a_circle_and_need_the_same_type() {
    let mut engine = Engine::default();
    let moved = engine.add_program(moved());
    let (mut a, mut b, mut c) = (engine.instantiate(moved), engine.instantiate(moved), engine.instantiate(moved));
    engine.link(&mut b, "values", &a, "moved").unwrap();
    engine.link(&mut c, "values", &b, "moved").unwrap();
    assert_eq!(engine.link(&mut a, "values", &c, "moved"), Err(LinkError::Cycle));
    assert!(matches!(engine.link(&mut a, "nothing", &c, "moved"), Err(LinkError::UnknownParam(_))));

    // A chain, given in any order, is planned from its start.
    engine.set_param(&mut a, "values", &[1.0f32]).unwrap();
    assert_eq!(engine.evaluate([&mut c, &mut b, &mut a]), 3);
    assert_eq!(engine.read::<f32>(&c, "moved"), Some(&[2.5][..]));
    // Once it is unlinked, the link from `b` is gone.
    engine.unlink(&mut c, "values");
    engine.set_param(&mut a, "values", &[2.0f32]).unwrap();
    assert_eq!(engine.evaluate([&mut c, &mut b, &mut a]), 2);
    assert_eq!(engine.read::<f32>(&c, "moved"), Some(&[2.5][..]));
    engine.link(&mut a, "values", &c, "moved").unwrap();

    let mut b = ProgramBuilder::new("counts");
    let counts = b.param::<u32>("counts", &[]);
    b.output("counts", counts);
    let counts = engine.add_program(b.finish());
    let counts = engine.instantiate(counts);
    assert!(matches!(engine.link(&mut c, "values", &counts, "counts"), Err(LinkError::Type { .. })));
}
