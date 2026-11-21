use oneiroi_core::node::NodeRegistry;

#[doc(hidden)]
pub use oneiroi_core as __core;
#[cfg(feature = "meta")]
#[doc(hidden)]
pub use oneiroi_meta as __meta;
#[cfg(feature = "meta")]
pub use oneiroi_meta as meta;
#[cfg(feature = "meta")]
#[doc(hidden)]
pub use paste as __paste;

#[macro_use]
mod describe;

pub mod curve;
mod generic;
pub use generic::with_defaults;
pub mod instance;
mod parallel;
pub mod primitive;
pub mod property;
pub mod surface;
pub mod value;
#[cfg(feature = "meta")]
pub mod gizmo;

/// Makes every node of this crate known to `registry`.
pub fn register(registry: &mut NodeRegistry) {
    curve::register(registry);
    value::register(registry);
    instance::register(registry);
}

/// A data type with what people see of it, see [`describe_type!`].
#[cfg(feature = "meta")]
pub trait Described {
    const META: oneiroi_meta::TypeMeta;
}

/// Makes what people see of every node, type and trait of this crate known
/// to `meta`.
#[cfg(feature = "meta")]
pub fn describe(meta: &mut oneiroi_meta::MetaRegistry) {
    use curve::{EvaluatedFrame, SweptPatch, TubePatch, nurbs::CubicNurbsSegmentCache};
    use glam::{Vec2, Vec3};

    for node in [
        &value::F32_META,
        &value::VEC3_META,
        &value::MATH_META,
        &value::COMBINE_XYZ_META,
        &curve::NURBS_META,
        &curve::SAMPLE_CURVE_META,
        &curve::TRANSLATE_META,
        &curve::SWEEP_META,
        &primitive::CIRCLE_META,
        &primitive::RECTANGLE_META,
        &curve::LINE_META,
        &curve::POLYLINE_META,
        &curve::TO_CURVE_META,
        &curve::TO_FRAMES_META,
        &curve::CURVE_2D_META,
        &surface::MESH_META,
        &instance::INSTANCE_META,
        &instance::TRANSFORM_META,
        &instance::REALIZE_META,
    ] {
        meta.add_node(*node);
    }
    for ty in [
        f32::META,
        u32::META,
        Vec2::META,
        Vec3::META,
        curve::ControlPoint::META,
        CubicNurbsSegmentCache::META,
        EvaluatedFrame::META,
        primitive::Circle::META,
        primitive::Rectangle::META,
        TubePatch::META,
        SweptPatch::META,
    ] {
        meta.add_type(ty);
    }
    meta.add_trait(curve::PROFILE_META);
    meta.add_trait(curve::OUTLINE_META);
    meta.add_trait(surface::SURFACE_META);
    meta.add_trait(instance::PLACEABLE_META);
    instance::describe(meta);
    gizmo::describe(meta);
}
