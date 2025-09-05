use std::fmt::Display;

#[derive(Clone)]
pub struct Tile(pmtiles::TileCoord);

impl std::ops::Deref for Tile {
    type Target = pmtiles::TileCoord;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.z(), self.x(), self.y())
    }
}

impl From<pmtiles::TileCoord> for Tile {
    fn from(tc: pmtiles::TileCoord) -> Self {
        Self(tc)
    }
}

impl From<pmtiles::TileId> for Tile {
    fn from(tid: pmtiles::TileId) -> Self {
        Self(pmtiles::TileCoord::from(tid))
    }
}
