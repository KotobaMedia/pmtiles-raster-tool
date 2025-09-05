use std::str::FromStr;

use anyhow::{Error, anyhow};

mod gsidem_terrainrgb;
mod shared;

pub use shared::TransformProcess;

/// Supported transforms
#[derive(Clone, Debug)]
pub enum Transform {
    /// Transform Japan's GSI DEM PNG format to Mapbox TerrainRGB tiles
    GsiDemPngToTerrainRgbPng(gsidem_terrainrgb::GsiDemPngToTerrainRgbPng),
}

impl FromStr for Transform {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "gsidempng-to-terrainrgbpng" => Ok(Self::GsiDemPngToTerrainRgbPng(
                gsidem_terrainrgb::GsiDemPngToTerrainRgbPng::new(),
            )),
            _ => Err(anyhow!(
                "invalid transform: {s}. valid values: gsidempng-to-terrainrgbpng"
            )),
        }
    }
}

impl TransformProcess for Transform {
    fn new() -> Self
    where
        Self: Sized,
    {
        panic!("Transform::new() should not be called directly");
    }

    fn transform(&self, input: &[u8]) -> anyhow::Result<bytes::Bytes> {
        match self {
            Transform::GsiDemPngToTerrainRgbPng(t) => t.transform(input),
        }
    }
}
