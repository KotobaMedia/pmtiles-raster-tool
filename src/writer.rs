use std::{collections::BTreeMap, fs::File, path::PathBuf};

use anyhow::{Context, Result};
use bytes::Bytes;
use flume::Receiver;
use pmtiles::{PmTilesStreamWriter, PmTilesWriter};

use crate::reader::PmTilesReader;

pub struct WriteTileMsg {
    pub index: usize,
    pub tile_id: pmtiles::TileId,
    pub tile_data: Bytes,
}

pub struct Writer {
    output: PathBuf,
    out_pmt: PmTilesStreamWriter<File>,
}

impl Writer {
    pub async fn new(output: PathBuf, force: bool, in_pmt: PmTilesReader) -> Result<Self> {
        let out_pmt_f = File::options()
            .create_new(!force)
            .write(true)
            .open(&output)
            .context("Failed to open output file. Hint: try specifying --force if you want to overwrite an existing file.")?;
        out_pmt_f.set_len(0)?;

        let header = in_pmt.get_header();
        let metadata = in_pmt.get_metadata().await?;
        let out_pmt = PmTilesWriter::new(header.tile_type)
            .tile_compression(header.tile_compression)
            .min_zoom(header.min_zoom)
            .max_zoom(header.max_zoom)
            .bounds(
                header.min_longitude,
                header.min_latitude,
                header.max_longitude,
                header.max_latitude,
            )
            .center_zoom(header.center_zoom)
            .center(header.center_longitude, header.center_latitude)
            .metadata(&metadata)
            .create(out_pmt_f)?;

        Ok(Self { output, out_pmt })
    }

    pub fn write(mut self, tile_rx: Receiver<WriteTileMsg>) -> Result<()> {
        let mut next = 0usize;
        // reorder buffer
        // TODO: use a more efficient structure
        let mut buf = BTreeMap::new();
        for msg in tile_rx {
            buf.insert(msg.index, msg);
            while let Some(msg) = buf.remove(&next) {
                self.out_pmt.add_tile(msg.tile_id.into(), &msg.tile_data)?;
                next += 1;
            }
        }
        println!("Finished writing tiles, finalizing archive...");
        self.out_pmt.finalize()?;
        println!("Finished writing to {}.", self.output.display());
        Ok(())
    }
}
