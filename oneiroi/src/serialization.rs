use std::time::Instant;

use crate::asset::Asset;

pub fn serialize_asset_v1(asset: &Asset) -> String {
    match asset {
        Asset::Editable(_) => serde_json::to_string_pretty(&asset).expect("Serialization failed"),
        Asset::Runtime(_) => todo!(),
    }
}

//bundle all external dependencies in subgraphs
pub fn export_asset_bundled() {}

pub fn deserialize_asset_v1(asset: &str) -> Asset {
    let computation_instant = Instant::now();

    let p: Asset = serde_json::from_str(asset).expect("Couldnt parse file into asset");
    println!(
        "Asset serialization took {} nanoseconds",
        computation_instant.elapsed().as_nanos()
    );
    p
}
