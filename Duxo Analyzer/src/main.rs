use anyhow::Result;
use duxo_analyzer::{profile_game, scanner::analyze_game};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct AnalysisReport {
    #[serde(flatten)]
    manifest: duxo_analyzer::GameManifest,
    profile: duxo_analyzer::GameProfile,
}

fn main() -> Result<()> {
    let input = std::env::args_os().nth(1).map(PathBuf::from);

    match input {
        Some(path) => {
            let manifest = analyze_game(&path)?;
            let profile = profile_game(&manifest)?;
            let report = AnalysisReport { manifest, profile };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        None => {
            eprintln!("Usage: duxo-analyzer <game-directory>");
            std::process::exit(2);
        }
    }

    Ok(())
}
