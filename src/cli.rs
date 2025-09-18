use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use clap::Parser;

use crate::transform::Transform;

/// CLI definition matching README usage:
/// pmtiles-raster-tool in.pmtiles transform out.pmtiles
#[derive(Debug, Parser)]
#[command(name = "pmtiles-raster-tool")]
#[command(about = "A tool to transform raster tiles", version)]
pub struct Cli {
    /// Input PMTiles file path
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    /// Transform to apply
    #[arg(value_name = "TRANSFORM")]
    pub transform: Transform,

    /// Output PMTiles file path
    #[arg(value_name = "OUTPUT")]
    pub output: PathBuf,

    #[arg(long, short, help = "Overwrite output if it already exists")]
    pub force: bool,
}

/// Resolved, strongly-typed arguments
#[derive(Debug)]
pub struct ResolvedCli {
    pub input: PathBuf,
    pub transform: Transform,
    pub output: PathBuf,
    pub force: bool,
}

impl Cli {
    /// Parse args and resolve optional transform vs output positionally.
    pub fn parse_resolved() -> Result<ResolvedCli> {
        let cli = Self::parse();
        // Transform is now a required positional argument
        // Validate transform via FromStr (clap already invokes it)
        // But ensure we fail early if somehow empty
        let _ = Transform::from_str(
            &(match cli.transform {
                Transform::GsiDemPngToTerrainRgbPng(_) => "gsidempng-to-terrainrgbpng",
            })
            .to_string(),
        )
        .map_err(|e| anyhow!(e))?;

        Ok(ResolvedCli {
            input: cli.input,
            transform: cli.transform,
            output: cli.output,
            force: cli.force,
        })
    }
}
