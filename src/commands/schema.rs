use anyhow::{Context, Result};
use fs_err as fs;

use crate::config::load_config;
use crate::schema::CONFIG_SCHEMA_JSON;
use crate::util::development_dir;

pub fn run() -> Result<()> {
    let config = load_config()?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = development_dir(dev)?;
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("ディレクトリ作成に失敗しました: {}", install_dir.display()))?;
    let target = std::env::current_dir()
        .context("カレントディレクトリの取得に失敗しました")?
        .join(".aviutl2-cli")
        .join("aviutl2.schema.json");
    fs::create_dir_all(target.parent().unwrap_or(&install_dir))
        .with_context(|| format!("ディレクトリ作成に失敗しました: {}", target.display()))?;
    fs::write(&target, CONFIG_SCHEMA_JSON)
        .with_context(|| format!("JSON Schema の書き込みに失敗しました: {}", target.display()))?;
    log::info!("JSON Schema を出力しました: {}", target.display());
    Ok(())
}
