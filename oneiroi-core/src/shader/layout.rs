//! How values of each type are declared, read out of the pages of the
//! store and written back, in WGSL.

use std::fmt::Write;

use crate::types::{Field, FieldType, Layout, TypeInfo};

/// The expression which reads a field out of `page` at word `base + offset`.
fn read(ty: FieldType, page: &str, base: &str, offset: u32) -> String {
    let word = |index: u32| format!("{page}[{base} + {}u]", offset + index);
    let float = |index: u32| format!("bitcast<f32>({})", word(index));
    let floats = |from: u32, count: u32| {
        let parts: Vec<_> = (from..from + count).map(float).collect();
        parts.join(", ")
    };
    match ty {
        FieldType::U32 => word(0),
        FieldType::F32 => float(0),
        FieldType::Vec2 => format!("vec2<f32>({})", floats(0, 2)),
        FieldType::Vec3 => format!("vec3<f32>({})", floats(0, 3)),
        FieldType::Vec4 => format!("vec4<f32>({})", floats(0, 4)),
        FieldType::Mat4 => {
            let columns: Vec<_> = (0..4)
                .map(|column| format!("vec4<f32>({})", floats(column * 4, 4)))
                .collect();
            format!("mat4x4<f32>({})", columns.join(", "))
        }
        FieldType::Struct { wgsl, fields } => read_struct(wgsl, fields, page, base, offset),
        FieldType::Pad(_) => unreachable!("padding is never read"),
    }
}

/// The expression which builds a struct out of its fields.
fn read_struct(wgsl: &str, fields: &[Field], page: &str, base: &str, start: u32) -> String {
    let mut offset = start;
    let mut parts = Vec::new();
    for field in fields {
        if field.ty.wgsl().is_some() {
            parts.push(read(field.ty, page, base, offset));
        }
        offset += field.ty.words();
    }
    format!("{wgsl}({})", parts.join(", "))
}

fn write_struct(out: &mut String, fields: &[Field], value: &str, page: &str, base: &str, start: u32) {
    let mut offset = start;
    for field in fields {
        let value = format!("{value}.{}", field.name);
        write(out, field.ty, &value, page, base, offset);
        offset += field.ty.words();
    }
}

/// The statements which write `value` to `page` at word `base + offset`.
fn write(out: &mut String, ty: FieldType, value: &str, page: &str, base: &str, offset: u32) {
    let mut word = |index: u32, value: String| {
        let _ = writeln!(out, "    {page}[{base} + {}u] = {value};", offset + index);
    };
    match ty {
        FieldType::U32 => word(0, value.into()),
        FieldType::F32 => word(0, format!("bitcast<u32>({value})")),
        FieldType::Vec2 | FieldType::Vec3 | FieldType::Vec4 => {
            for index in 0..ty.words() {
                word(index, format!("bitcast<u32>({value}[{index}])"));
            }
        }
        FieldType::Mat4 => {
            for index in 0..16 {
                let (column, row) = (index / 4, index % 4);
                word(index, format!("bitcast<u32>({value}[{column}][{row}])"));
            }
        }
        FieldType::Pad(words) => {
            for index in 0..words {
                word(index, "0u".into());
            }
        }
        FieldType::Struct { fields, .. } => write_struct(out, fields, value, page, base, offset),
    }
}

/// The expression which reads a whole element of `ty`.
pub(crate) fn read_element(ty: &TypeInfo, page: &str, base: &str) -> String {
    match ty.layout {
        Layout::Value(field) => read(field, page, base, 0),
        Layout::Struct(fields) => read_struct(ty.wgsl, fields, page, base, 0),
    }
}

pub(super) fn write_element(out: &mut String, ty: &TypeInfo, value: &str, page: &str, base: &str) {
    match ty.layout {
        Layout::Value(field) => write(out, field, value, page, base, 0),
        Layout::Struct(fields) => write_struct(out, fields, value, page, base, 0),
    }
}

fn declare_struct(out: &mut String, declared: &mut Vec<&'static str>, wgsl: &'static str, fields: &'static [Field]) {
    if declared.contains(&wgsl) {
        return;
    }
    declared.push(wgsl);
    // A struct has to come after the ones it is made of.
    for field in fields {
        if let FieldType::Struct { wgsl, fields } = field.ty {
            declare_struct(out, declared, wgsl, fields);
        }
    }
    let _ = writeln!(out, "struct {wgsl} {{");
    for field in fields {
        if let Some(ty) = field.ty.wgsl() {
            let _ = writeln!(out, "    {}: {ty},", field.name);
        }
    }
    let _ = writeln!(out, "}}\n");
}

/// Declares the struct of `ty` and the ones inside of it, each only once.
pub(crate) fn declare(out: &mut String, declared: &mut Vec<&'static str>, ty: &TypeInfo) {
    if let Layout::Struct(fields) = ty.layout {
        declare_struct(out, declared, ty.wgsl, fields);
    }
}
