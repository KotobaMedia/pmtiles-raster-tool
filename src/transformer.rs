use anyhow::{Context, Result};
use flume::{Receiver, Sender};
use rayon::prelude::*;

use crate::{
    progress::{ProgressMsg, ProgressSender},
    reader::ReadTileMsg,
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

    pub fn run(
        &self,
        input: Receiver<ReadTileMsg>,
        output: Sender<WriteTileMsg>,
        progress_tx: ProgressSender,
    ) -> Result<()> {
        input.into_iter().par_bridge().try_for_each_with(
            (output, self.transform.clone()),
            |(output, transform), msg| {
                let transformed_data = transform
                    .transform(&msg.tile_data)
                    .with_context(|| format!("while transforming tile {}", msg.tile.to_string()))?;
                output.send(WriteTileMsg {
                    index: msg.index,
                    tile: msg.tile.clone(),
                    tile_data: transformed_data,
                })?;
                progress_tx
                    .send(ProgressMsg::Processed(msg.tile))
                    .context("Failed to send progress message")?;
                Ok::<(), anyhow::Error>(())
            },
        )?;
        Ok(())
    }
}
