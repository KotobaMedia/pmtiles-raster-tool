use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use bytes::Bytes;
use flume::Sender;
use futures_util::TryStreamExt;
use pmtiles::{AsyncPmTilesReader, MmapBackend};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::task::JoinSet;

use crate::{
    progress::{ProgressMsg, ProgressSender},
    tile::Tile,
};

pub type PmTilesReader = Arc<AsyncPmTilesReader<MmapBackend>>;

pub struct ReadTileMsg {
    pub index: usize,
    pub tile: Tile,
    pub tile_data: Bytes,
}

pub struct Reader {
    input: PathBuf,
    reader: PmTilesReader,
}

impl Reader {
    pub async fn new(input: PathBuf) -> Result<Self> {
        let reader = AsyncPmTilesReader::new_with_path(&input).await?;
        Ok(Self {
            input,
            reader: Arc::new(reader),
        })
    }

    pub async fn run(
        self,
        tile_tx: Sender<ReadTileMsg>,
        progress_tx: ProgressSender,
    ) -> Result<()> {
        let entries = self
            .reader
            .clone()
            .entries()
            .try_collect::<Vec<_>>()
            .await?;
        let mut coords = entries
            .iter()
            .flat_map(|e| e.iter_coords())
            .collect::<Vec<_>>();
        coords.sort_unstable();
        let coords_count = coords.len();
        progress_tx.send(ProgressMsg::UpdateCount(coords_count as u64))?;
        progress_tx.send(ProgressMsg::Log(format!(
            "Found {} tiles in the input: {}",
            coords_count,
            self.input.display()
        )))?;

        // Fetch tiles concurrently with a fixed-size async worker pool to avoid per-tile task overhead.
        let concurrency = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let coords = Arc::new(coords);
        let len = coords.len();
        let next_index = Arc::new(AtomicUsize::new(0));

        let mut join_set: JoinSet<anyhow::Result<()>> = JoinSet::new();
        for _ in 0..concurrency {
            let reader = self.reader.clone();
            let tile_tx = tile_tx.clone();
            let coords = coords.clone();
            let next_index = next_index.clone();
            join_set.spawn(async move {
                loop {
                    let i = next_index.fetch_add(1, Ordering::Relaxed);
                    if i >= len {
                        break;
                    }
                    // Assuming coords are Copy; if not, change to clone()
                    let coord = coords[i];
                    if let Some(tile_data) = reader.get_tile(coord).await? {
                        tile_tx
                            .send_async(ReadTileMsg {
                                index: i,
                                tile: coord.into(),
                                tile_data,
                            })
                            .await?;
                    }
                }
                Ok(())
            });
        }

        // Await all workers
        while let Some(res) = join_set.join_next().await {
            res??;
        }

        Ok(())
    }

    pub fn pmtiles_reader(&self) -> Arc<AsyncPmTilesReader<MmapBackend>> {
        self.reader.clone()
    }
}
