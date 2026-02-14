use anyhow::{Context, Result};
use fs_err as fs;

use crate::schema::CONFIG_SCHEMA_JSON;

pub fn run() -> Result<()> {
    let target = std::env::current_dir()
        .context("カレントディレクトリの取得に失敗しました")?
        .join(".aviutl2-cli")
        .join("aviutl2.schema.json");
    fs::create_dir_all(target.parent().unwrap())
        .with_context(|| format!("ディレクトリ作成に失敗しました: {}", target.display()))?;
    fs::write(&target, CONFIG_SCHEMA_JSON)
        .with_context(|| format!("JSON Schema の書き込みに失敗しました: {}", target.display()))?;
    log::info!("JSON Schema を出力しました: {}", target.display());
    Ok(())
}
