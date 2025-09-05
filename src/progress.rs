use anyhow::Result;
use flume::Receiver;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::tile::Tile;

pub enum ProgressMsg {
    Log(String),

    /// The tile count has been updated.
    UpdateCount(u64),

    /// A tile was processed.
    Processed(Tile),

    /// A tile was written.
    Written(Tile),

    /// All done.
    Finished(),
}

pub type ProgressSender = flume::Sender<ProgressMsg>;

pub struct Progress {
    m: MultiProgress,
    tile_processed: ProgressBar,
    tile_written: ProgressBar,
}

impl Progress {
    pub fn new() -> Self {
        let m = MultiProgress::new();

        let tile_processed = m.add(ProgressBar::new(0));
        tile_processed.set_style(
            ProgressStyle::with_template(
                "Process {msg} {bar:40.cyan/blue} {pos:>11}/{len:11} ({percent}%) ({per_sec}, {eta})",
            )
            .unwrap(),
        );

        let tile_written = m.add(ProgressBar::new(0));
        tile_written.set_style(
            ProgressStyle::with_template(
                "TileOut {msg} {bar:40.cyan/blue} {pos:>11}/{len:11} ({percent}%) ({per_sec}, {eta})",
            )
            .unwrap(),
        );

        Self {
            m,
            tile_processed,
            tile_written,
        }
    }

    pub fn run(&self, rx: Receiver<ProgressMsg>) -> Result<()> {
        while let Ok(msg) = rx.recv() {
            match msg {
                ProgressMsg::Log(s) => {
                    self.m.println(s)?;
                }
                ProgressMsg::UpdateCount(count) => {
                    self.tile_written.set_length(count);
                    self.tile_processed.set_length(count);
                }
                ProgressMsg::Processed(tile) => {
                    self.tile_processed.inc(1);
                    let tile_str = format!("{:<14}", tile.to_string());
                    self.tile_processed.set_message(tile_str);
                }
                ProgressMsg::Written(tile) => {
                    self.tile_written.inc(1);
                    let tile_str = format!("{:<14}", tile.to_string());
                    self.tile_written.set_message(tile_str);
                }
                ProgressMsg::Finished() => {
                    self.tile_written.finish();
                    self.tile_processed.finish();
                    break;
                }
            }
        }
        Ok(())
    }
}
