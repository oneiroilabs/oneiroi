use serde::{Deserialize, Serialize};

use crate::type_system::data_types::Color;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Texture {
    albedo: Color,
}

impl Texture {
    pub fn get_albedo(&self) -> Color {
        println!("{self:?}");
        self.albedo
    }
}
