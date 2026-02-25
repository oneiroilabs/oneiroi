use petgraph::{Directed, Graph};
use serde::{Deserialize, Serialize};

//TODO the runtime is for the player and should always be used when running the game
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RuntimeAsset {
    //graph: Graph<, (), Directed, u16>,
    todo: i64,
}
