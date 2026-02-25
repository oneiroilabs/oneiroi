use dashmap::DashMap;
use im::{GenericHashMap, GenericHashSet, GenericVector, shared_ptr::RcK};
use rustc_hash::FxBuildHasher;

pub mod asset;
pub mod nodes;
pub mod property;
pub mod serialization;
pub mod type_system;

pub type ImVec<T> = GenericVector<T, RcK>;
pub type ImHashMap<K, V> = GenericHashMap<K, V, FxBuildHasher, RcK>;
pub type ImHashSet<K> = GenericHashSet<K, FxBuildHasher, RcK>;

pub type ParHashMap<K, V> = DashMap<K, V, FxBuildHasher>;
