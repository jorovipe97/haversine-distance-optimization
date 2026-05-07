use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct HaversinePair {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HaversineData {
    pub pairs: Vec<HaversinePair>,
}
