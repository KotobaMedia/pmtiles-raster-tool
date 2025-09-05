use anyhow::Result;

mod cli;
mod reader;
mod transform;
mod transformer;
mod writer;

use cli::Cli;
use tokio::task::JoinSet;

use crate::{reader::ReadTileMsg, transformer::Transformer, writer::WriteTileMsg};

const QUEUE_CAPACITY: usize = 2_usize.pow(16);

#[tokio::main]
async fn main() -> Result<()> {
    let resolved = Cli::parse_resolved()?;

    let (reader_tx, reader_rx) = flume::bounded::<ReadTileMsg>(QUEUE_CAPACITY);
    let (writer_tx, writer_rx) = flume::bounded::<WriteTileMsg>(QUEUE_CAPACITY);

    let mut js = JoinSet::new();
    let reader = reader::Reader::new(resolved.input.clone()).await?;
    let transformer = Transformer::new(resolved.transform);
    let writer =
        writer::Writer::new(resolved.output.clone(), false, reader.pmtiles_reader()).await?;

    js.spawn(async move { reader.run(reader_tx).await });
    js.spawn_blocking(move || transformer.run(reader_rx, writer_tx));
    js.spawn_blocking(move || writer.write(writer_rx));

    while let Some(res) = js.join_next().await {
        res??;
    }
    Ok(())
}
