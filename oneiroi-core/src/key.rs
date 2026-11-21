//! The keys values are known by: what they are made of, not where they
//! are. Two values with the same key are interchangeable, which is how
//! instances, graphs and frames share their results.

use std::{collections::HashMap, hash::{Hash, Hasher}};

use xxhash_rust::xxh3::{Xxh3, xxh3_128};

use crate::types::TypeInfo;

/// Identifies a value by how it was made instead of where it lives.
///
/// Two values with the same key are interchangeable, which is what lets
/// instances with equal parameters share their results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentKey(u128);

/// A key is a hash already, evenly spread over all its bits, so a map
/// takes some of them as they are instead of hashing it again.
impl Hash for ContentKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0 as u64);
    }
}

impl nohash_hasher::IsEnabled for ContentKey {}

/// A map by [`ContentKey`], which does not hash the keys again.
pub type KeyMap<V> = HashMap<ContentKey, V, nohash_hasher::BuildNoHashHasher<ContentKey>>;

/// A set of [`ContentKey`]s, which does not hash them again.
pub type KeySet = std::collections::HashSet<ContentKey, nohash_hasher::BuildNoHashHasher<ContentKey>>;

impl ContentKey {
    /// Stands in for a key which is not derived yet.
    pub(crate) const UNSET: Self = Self(0);

    /// The key of a leaf value: it is identified by its type and its bytes.
    pub fn of_data(ty: &TypeInfo, words: &[u32]) -> Self {
        // What the streaming hasher would be given, see `KeyHasher`. XXH3
        // comes to the same over the same bytes either way, and small
        // values, like the elements of most groups, are hashed at once.
        const ON_STACK: usize = 1024;
        let len = 1 + ty.name.len() + 1 + size_of::<usize>() + 4 * words.len();
        if len > ON_STACK {
            let mut hasher = KeyHasher::new(0);
            hasher.write(ty.name);
            hasher.write(words);
            return hasher.finish();
        }
        let mut bytes = [0u8; ON_STACK];
        let mut at = 0;
        let mut put = |part: &[u8]| {
            bytes[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        };
        put(&[0]);
        put(ty.name.as_bytes());
        put(&[0xff]);
        put(&words.len().to_ne_bytes());
        put(bytemuck::cast_slice(words));
        ContentKey(xxh3_128(&bytes[..len]))
    }

    /// [`ContentKey::derive`] for a node: its results change when its
    /// version does.
    pub fn derive_node(node: &str, version: u32, output: u32, inputs: &[ContentKey]) -> Self {
        derived(2, node, &[version as u64, output as u64], inputs)
    }

    /// The key of the `output`-th result of running `node` on `inputs`.
    pub fn derive(node: &str, output: u32, inputs: &[ContentKey]) -> Self {
        derived(1, node, &[output as u64], inputs)
    }

    /// The key of the `output`-th result of running the program with the
    /// hash `program` for each element `element` of `inputs`, at most `max`
    /// times.
    pub fn derive_for_each(program: u64, element: u32, max: u32, output: u32, inputs: &[ContentKey]) -> Self {
        derived(3, "for each", &[program, element as u64, max as u64, output as u64], inputs)
    }
}

/// The key of whatever comes out of `inputs` by what `name` and `numbers`
/// say. The few bytes this takes are put together on the stack and hashed
/// at once, which costs a fraction of setting up a streaming hasher: keys
/// are derived for every node of every element.
fn derived(domain: u8, name: &str, numbers: &[u64], inputs: &[ContentKey]) -> ContentKey {
    const ON_STACK: usize = 512;
    let len = 1 + 8 + name.len() + 8 * numbers.len() + 8 + 16 * inputs.len();
    let mut stack = [0u8; ON_STACK];
    let mut heap = Vec::new();
    let bytes = match len <= ON_STACK {
        true => &mut stack[..len],
        false => {
            heap.resize(len, 0);
            &mut heap[..]
        }
    };
    let mut at = 0;
    let mut put = |part: &[u8]| {
        bytes[at..at + part.len()].copy_from_slice(part);
        at += part.len();
    };
    put(&[domain]);
    put(&(name.len() as u64).to_le_bytes());
    put(name.as_bytes());
    for number in numbers {
        put(&number.to_le_bytes());
    }
    put(&(inputs.len() as u64).to_le_bytes());
    for input in inputs {
        put(&input.0.to_le_bytes());
    }
    ContentKey(xxh3_128(bytes))
}

/// XXH3 with 128 bits: fast over the megabytes a parameter can have, and
/// with too many bits for two different values to meet by chance. Keys are
/// never shared with anybody, so nothing has to resist an attacker.
pub(crate) struct KeyHasher(pub(crate) Xxh3);

impl KeyHasher {
    pub(crate) fn new(domain: u8) -> Self {
        let mut hasher = Self(Xxh3::new());
        hasher.write(domain);
        hasher
    }

    pub(crate) fn write(&mut self, value: impl Hash) {
        value.hash(&mut self.0);
    }

    pub(crate) fn finish(self) -> ContentKey {
        ContentKey(self.0.digest128())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataType;

    fn key(n: u32) -> ContentKey {
        ContentKey::of_data(&u32::INFO, &[n])
    }

    #[test]
    fn keys_separate_type_content_and_derivation() {
        assert_eq!(key(1), key(1));
        assert_ne!(key(1), key(2));
        assert_ne!(
            ContentKey::of_data(&u32::INFO, &[0]),
            ContentKey::of_data(&f32::INFO, &[0])
        );
        assert_ne!(
            ContentKey::derive("a", 0, &[key(1)]),
            ContentKey::derive("a", 1, &[key(1)])
        );
        assert_ne!(
            ContentKey::derive("a", 0, &[key(1)]),
            ContentKey::derive("b", 0, &[key(1)])
        );
    }

    #[test]
    fn derived_keys_depend_on_every_bit_of_their_inputs() {
        let low = ContentKey(1);
        let high = ContentKey(1 | 1 << 100);
        assert_ne!(ContentKey::derive("a", 0, &[low]), ContentKey::derive("a", 0, &[high]));
        // However long the inputs are.
        let many = |last| [vec![ContentKey(7); 40], vec![last]].concat();
        assert_ne!(ContentKey::derive("a", 0, &many(low)), ContentKey::derive("a", 0, &many(high)));
        // Names and numbers do not run into each other.
        assert_ne!(ContentKey::derive("a", 0, &[]), ContentKey::derive("a\0", 0, &[]));
    }
}
