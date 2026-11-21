use super::*;
use crate::{store::Store, types::{DataType, PortDefault}};

static PICK: NodeDef = NodeDef {
    id: "test.pick",
    version: 0,
    inputs: &[PortDecl::new::<u32>("mode").choices(&["a", "b", "c"])],
    outputs: &[PortDecl::new::<u32>("mode")],
    cpu: |inputs, outputs| outputs.set(0, &[inputs.choice(0) as u32]),
    gpu: &[],
};

#[test]
fn a_default_is_the_words_of_its_elements() {
    const END: PortDecl = PortDecl::new::<glam::Vec3>("end").with_default(&[glam::Vec3::new(1.0, 2.0, 3.0)]);
    let words: Vec<u32> = [1.0f32, 2.0, 3.0].map(f32::to_bits).into();
    assert_eq!(END.default.words(), words);
    assert!(END.default.fits(&glam::Vec3::INFO) && !END.default.fits(&f32::INFO));
    assert!(PortDefault::EMPTY.fits(&f32::INFO));
}

#[test]
fn a_choice_starts_at_the_first_and_ends_at_the_last() {
    let port = &PICK.inputs[0];
    assert_eq!((port.card, port.default.words()), (Card::One, &[0][..]));

    let mut store = Store::new();
    for (value, picked) in [(0, 0), (2, 2), (7, 2)] {
        let key = crate::store::ContentKey::of_data(&u32::INFO, &[value]);
        let handles = [store.insert(key, &[value])];
        let inputs = Inputs::new(&store, &PICK, &handles);
        assert_eq!(inputs.choice(0), picked);
    }
    NodeRegistry::new().register(&PICK);
}

#[test]
fn a_conversion_is_found_by_what_it_makes() {
    static TO_U32: GenericNode = GenericNode {
        id: "test.to_u32",
        inputs: &[GenericPort::exact::<f32>("values")],
        outputs: &[GenericPort::exact::<u32>("values")],
    };
    static F32_TO_U32: NodeDef = NodeDef {
        id: "test.to_u32.f32",
        version: 0,
        inputs: &[PortDecl::new::<f32>("values")],
        outputs: &[PortDecl::new::<u32>("values")],
        cpu: |_, _| {},
        gpu: &[],
    };
    let mut registry = NodeRegistry::new();
    registry.register_instance(&TO_U32, &F32_TO_U32);
    assert!(!registry.converts(&f32::INFO));
    registry.add_conversion(&TO_U32);
    registry.add_conversion(&TO_U32);

    let (conversion, to) = registry.conversion(&f32::INFO, |ty| *ty == u32::INFO).unwrap();
    assert_eq!((conversion.id, to), ("test.to_u32", u32::INFO));
    assert!(registry.conversion(&f32::INFO, |ty| *ty == f32::INFO).is_none());
    assert!(registry.conversion(&u32::INFO, |_| true).is_none());
}

#[test]
#[should_panic(expected = "has to be a single `u32`")]
fn only_a_single_u32_has_choices() {
    static WRONG: NodeDef = NodeDef {
        id: "test.wrong",
        version: 0,
        inputs: &[PortDecl::new::<f32>("mode").choices(&["a"])],
        outputs: &[],
        cpu: |_, _| {},
        gpu: &[],
    };
    NodeRegistry::new().register(&WRONG);
}
