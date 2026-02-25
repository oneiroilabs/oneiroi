use std::ops::{Deref, DerefMut};

use godot::prelude::*;
use oneiroi::asset::Asset;

#[derive(GodotClass)]
#[class(tool, init, base=Resource)]
pub struct OneiroiAsset {
    base: Base<Resource>,

    asset: Asset,
}

#[godot_api]
impl OneiroiAsset {
    pub fn init_from_inner(asset: Asset) -> Gd<Self> {
        Gd::from_init_fn(|base| OneiroiAsset { base, asset })
    }
}

impl Deref for OneiroiAsset {
    type Target = Asset;

    fn deref(&self) -> &Self::Target {
        &self.asset
    }
}

impl DerefMut for OneiroiAsset {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asset
    }
}
