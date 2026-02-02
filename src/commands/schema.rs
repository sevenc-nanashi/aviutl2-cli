use anyhow::{Context, Result};
use fs_err as fs;
use std::path::PathBuf;

use crate::config::load_config;
use crate::schema::CONFIG_SCHEMA_JSON;

pub fn run() -> Result<()> {
    let config = load_config()?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = PathBuf::from(dev.install_dir.as_deref().unwrap_or("./development"));
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("ディレクトリ作成に失敗しました: {}", install_dir.display()))?;
    let target = install_dir.join("aviutl2.schema.json");
    fs::write(&target, CONFIG_SCHEMA_JSON)
        .with_context(|| format!("JSON Schema の書き込みに失敗しました: {}", target.display()))?;
    log::info!("JSON Schema を出力しました: {}", target.display());
    Ok(())
}
