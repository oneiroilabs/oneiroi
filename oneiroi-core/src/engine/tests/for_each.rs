use super::*;

/// `value * 10`, and how many values there were.
fn tenfold(engine: &mut Engine) -> ProgramId {
    let mut b = ProgramBuilder::new("tenfold");
    let value = b.param::<f32>("value", &[0.]);
    let ten = b.constant::<f32>(&[10.]);
    let out = b.node(&SCALE, &[value, ten]).unwrap()[0];
    let sum = b.node(&SUM, &[out, value]).unwrap();
    b.output("out", out);
    b.output("count", sum[1]);
    engine.add_program(b.finish())
}

fn each(engine: &mut Engine, max: u32) -> ProgramId {
    let inner = tenfold(engine);
    let mut b = ProgramBuilder::new("each");
    let values = b.param::<f32>("values", &[1., 2., 3.]);
    let out = b
        .for_each(engine.programs(), inner, &[Some(values)], 0, max)
        .unwrap();
    b.output("out", out[0]);
    b.output("count", out[1]);
    engine.add_program(b.finish())
}

#[test]
fn a_program_runs_for_each_element() {
    let mut engine = Engine::default();
    let id = each(&mut engine, 100);
    let mut instance = engine.instantiate(id);

    // Two nodes for each of the three elements.
    assert_eq!(engine.evaluate([&mut instance]), 6);
    assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[10., 20., 30.][..]));
    // Every run saw a single element.
    assert_eq!(engine.read::<u32>(&instance, "count"), Some(&[1, 1, 1][..]));
    assert!(engine.overflows().is_empty());
    assert_eq!(engine.evaluate([&mut instance]), 0);
}

#[test]
fn elements_which_stay_the_same_are_not_run_again() {
    let mut engine = Engine::default();
    let id = each(&mut engine, 100);
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);

    // One new element, and one which is there twice now.
    engine
        .set_param(&mut instance, "values", &[1f32, 2., 5., 5.])
        .unwrap();
    assert_eq!(engine.evaluate([&mut instance]), 2);
    assert_eq!(
        engine.read::<f32>(&instance, "out"),
        Some(&[10., 20., 50., 50.][..])
    );

    engine.set_param(&mut instance, "values", &[] as &[f32]).unwrap();
    assert_eq!(engine.evaluate([&mut instance]), 0);
    assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[][..]));
}

#[test]
fn more_elements_than_allowed_are_left_out_and_reported() {
    let mut engine = Engine::default();
    let id = each(&mut engine, 2);
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);

    assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[10., 20.][..]));
    assert_eq!(
        engine.overflows(),
        [Overflow {
            program: "tenfold".into(),
            elements: 3,
            max: 2
        }]
    );
    // Nothing ran the next time, so nothing overflowed.
    engine.evaluate([&mut instance]);
    assert!(engine.overflows().is_empty());
}

#[test]
fn calls_for_each_element_can_be_nested() {
    let mut engine = Engine::default();
    let each = each(&mut engine, 100);

    // For each list of values (here: twice the same), all of them.
    let mut b = ProgramBuilder::new("each of each");
    let values = b.param::<f32>("values", &[1., 2.]);
    let pairs = b.node(&SUM, &[values, values]).unwrap()[0];
    let out = b
        .for_each(engine.programs(), each, &[Some(pairs)], 0, 100)
        .unwrap();
    b.output("out", out[0]);
    let id = engine.add_program(b.finish());
    let mut instance = engine.instantiate(id);
    engine.evaluate([&mut instance]);

    // 2 and 4, each a list of one, times ten.
    assert_eq!(engine.read::<f32>(&instance, "out"), Some(&[20., 40.][..]));
    assert_eq!(engine.programs_mut().retain([id]), 0);
}

#[test]
fn something_has_to_be_given_to_take_the_elements_of() {
    let mut engine = Engine::default();
    let inner = tenfold(&mut engine);
    let mut b = ProgramBuilder::new("broken");
    assert!(matches!(
        b.for_each(engine.programs(), inner, &[None], 0, 1),
        Err(BuildError::Element { .. })
    ));
    assert!(matches!(
        b.for_each(engine.programs(), inner, &[None], 3, 1),
        Err(BuildError::Element { .. })
    ));
}
