use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use bytes::Bytes;
use flume::Sender;
use futures_util::TryStreamExt;
use pmtiles::{AsyncPmTilesReader, MmapBackend};

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

        let mut index = 0;
        for coord in coords {
            let Some(tile_data) = self.reader.get_tile(coord).await? else {
                continue;
            };
            tile_tx
                .send_async(ReadTileMsg {
                    index,
                    tile: coord.into(),
                    tile_data,
                })
                .await?;
            index += 1;
        }

        Ok(())
    }

    pub fn pmtiles_reader(&self) -> Arc<AsyncPmTilesReader<MmapBackend>> {
        self.reader.clone()
    }
}
