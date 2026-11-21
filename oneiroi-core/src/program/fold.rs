//! A program cut down to what depends on its parameters, with the values
//! the rest gave it as constants.

use std::{collections::HashMap, hash::{DefaultHasher, Hash, Hasher}};

use crate::{
    program::{Op, OutputDecl, ParamDecl, Program, Prototype, SlotDecl, SlotId, SlotSource},
    store::ContentKey,
};

impl Program {
    /// The slots a program cut down to the operations `keep` still needs:
    /// the parameters, the outputs, what the kept operations read and
    /// write, and what comes along with any of these, like groups.
    pub(crate) fn needed(&self, keep: &[bool]) -> Vec<bool> {
        let mut needed = vec![false; self.slots.len()];
        let mut open: Vec<SlotId> = self.params.iter().map(|param| param.slot).collect();
        open.extend(self.outputs.iter().map(|output| output.slot));
        for (op, _) in self.ops.iter().zip(keep).filter(|(_, keep)| **keep) {
            open.extend(op.inputs.iter().chain(op.outputs.iter()));
        }
        while let Some(slot) = open.pop() {
            if std::mem::replace(&mut needed[slot.0 as usize], true) {
                continue;
            }
            let decl = self.slot(slot);
            let prototype = decl.prototype.map(|p| [p.elements, p.groups]).into_iter().flatten();
            open.extend(decl.groups.into_iter().chain(prototype).chain(decl.total));
        }
        needed
    }

    /// This program with only the operations `keep`, and in place of every
    /// slot it still needs which no kept operation makes, the constant in
    /// `values`: what folding its constant part leaves of it. A constant
    /// keeps the key it was derived under, so the rest of the program finds
    /// the same results as before.
    ///
    /// Panics if a slot which is needed has no value.
    pub(crate) fn folded(&self, keep: &[bool], values: &HashMap<SlotId, (ContentKey, Box<[u32]>)>) -> Program {
        let needed = self.needed(keep);
        let mut slot_of = vec![None; self.slots.len()];
        let mut count = 0;
        for (index, needed) in needed.iter().enumerate() {
            if *needed {
                slot_of[index] = Some(SlotId(count));
                count += 1;
            }
        }
        let mut op_of = vec![None; self.ops.len()];
        let mut ops = 0;
        for (index, keep) in keep.iter().enumerate() {
            if *keep {
                op_of[index] = Some(ops);
                ops += 1;
            }
        }
        let map = |slot: SlotId| slot_of[slot.0 as usize].expect("a slot which is needed");

        let slots = self
            .slots
            .iter()
            .enumerate()
            .filter(|(index, _)| needed[*index])
            .map(|(index, decl)| {
                let source = match &decl.source {
                    SlotSource::Op { op, output } if keep[*op as usize] => SlotSource::Op {
                        op: op_of[*op as usize].unwrap(),
                        output: *output,
                    },
                    SlotSource::Op { .. } => {
                        let (key, words) = values
                            .get(&SlotId(index as u32))
                            .unwrap_or_else(|| panic!("slot {index} is needed and has no value"));
                        SlotSource::Const { words: words.clone(), key: *key }
                    }
                    SlotSource::Param(index) => SlotSource::Param(*index),
                    SlotSource::Const { words, key } => SlotSource::Const { words: words.clone(), key: *key },
                };
                SlotDecl {
                    ty: decl.ty,
                    source,
                    card: decl.card,
                    groups: decl.groups.map(map),
                    prototype: decl.prototype.map(|p| Prototype { elements: map(p.elements), groups: map(p.groups) }),
                    total: decl.total.map(map),
                }
            })
            .collect();
        let ops = self
            .ops
            .iter()
            .zip(keep)
            .filter(|(_, keep)| **keep)
            .map(|(op, _)| Op {
                kind: op.kind,
                inputs: op.inputs.iter().copied().map(map).collect(),
                outputs: op.outputs.iter().copied().map(map).collect(),
            })
            .collect();
        let mut hasher = DefaultHasher::new();
        (self.hash, "folded").hash(&mut hasher);
        Program {
            name: self.name.clone(),
            hash: hasher.finish(),
            slots,
            params: self
                .params
                .iter()
                .map(|param| ParamDecl {
                    name: param.name.clone(),
                    slot: map(param.slot),
                    default: param.default.clone(),
                    hidden: param.hidden,
                })
                .collect(),
            ops,
            outputs: self
                .outputs
                .iter()
                .map(|output| OutputDecl { name: output.name.clone(), slot: map(output.slot) })
                .collect(),
        }
    }
}
