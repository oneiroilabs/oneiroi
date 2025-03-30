use crate::asset::Asset;

// basic export
pub fn export_asset_v1(asset: &Asset) -> String {
    serde_json::to_string_pretty(&asset).expect("Serialization failed")
    //toml::to_string_pretty(&asset).expect("Serialization failed")
}

//bundle all external dependencies in subgraphs
pub fn export_asset_bundled() {}

// basic export
pub fn import_asset_v1(asset: String) -> Asset {
    let p: Asset = serde_json::from_str(&asset).expect("Couldnt parse file into asset");
    //let p: Asset = toml::from_str(&asset).expect("Couldnt parse file into asset");
    p
}
