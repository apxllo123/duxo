use anyhow::Context;
use duxo_adapter::plan::build_plan;
use duxo_analyzer::GameProfile;
use std::io::{self, Read};

fn main() -> anyhow::Result<()> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("failed to read GameProfile from stdin")?;
    let profile: GameProfile = serde_json::from_str(&input).context("invalid GameProfile JSON")?;
    println!("{}", serde_json::to_string_pretty(&build_plan(&profile))?);
    Ok(())
}
