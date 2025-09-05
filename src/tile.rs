pub fn tile_to_string(coord: pmtiles::TileCoord) -> String {
    format!("{}/{}/{}", coord.z(), coord.x(), coord.y())
}
