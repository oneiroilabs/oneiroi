use godot::prelude::*;
use oneiroi::asset::{Asset, instance::AssetInstance};

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

    pub fn get_inner(&self) -> &Asset {
        &self.asset
    }

    pub fn get_inner_mut(&mut self) -> &mut Asset {
        &mut self.asset
    }
}
