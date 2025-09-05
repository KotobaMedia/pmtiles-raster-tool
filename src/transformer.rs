use anyhow::{Context, Result};
use flume::{Receiver, Sender};
use rayon::prelude::*;

use crate::{
    reader::ReadTileMsg,
    tile::tile_to_string,
    transform::{Transform, TransformProcess},
    writer::WriteTileMsg,
};

/// The logic to run the transform processes in parallel and coordinate with the rest of the app.
pub struct Transformer {
    transform: Transform,
}

impl Transformer {
    /// Create a new transformer for the given transform
    pub fn new(transform: Transform) -> Self {
        Self { transform }
    }

    pub fn run(&self, input: Receiver<ReadTileMsg>, output: Sender<WriteTileMsg>) -> Result<()> {
        input.into_iter().par_bridge().try_for_each_with(
            (output, self.transform.clone()),
            |(output, transform), msg| {
                let transformed_data = transform.transform(&msg.tile_data).with_context(|| {
                    format!(
                        "while transforming tile {}",
                        tile_to_string(msg.tile_id.into())
                    )
                })?;
                output.send(WriteTileMsg {
                    index: msg.index,
                    tile_id: msg.tile_id,
                    tile_data: transformed_data,
                })?;
                Ok::<(), anyhow::Error>(())
            },
        )?;
        Ok(())
    }
}
