use anyhow::Context;
use fs_err as fs;
use std::io::{BufRead, Write};

pub fn run(
    opts: &crate::config::ConfigLoadOpts,
    reset_effects: bool,
    reset_movements: bool,
) -> anyhow::Result<()> {
    let config = crate::config::load_config(opts)?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = crate::util::development_dir(dev)?;

    let aviutl2_ini_path = install_dir.join("data").join("aviutl2.ini");
    if !aviutl2_ini_path.exists() {
        tracing::warn!(
            "aviutl2.ini が見つかりません: {}",
            aviutl2_ini_path.display()
        );
        return Ok(());
    }
    let mut new_aviutl2_ini = tempfile::NamedTempFile::new()?;
    let new_aviutl2_ini_file = new_aviutl2_ini.as_file_mut();
    let current_aviutl2_ini = std::io::BufReader::new(fs::File::open(&aviutl2_ini_path)?);

    let mut skipping = false;
    let mut n_removed_effects = 0;
    let mut n_removed_movements = 0;
    for line in current_aviutl2_ini.lines() {
        let line = line?;
        if let Some(section_name) = line
            .trim()
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
        {
            skipping = false;

            if reset_effects && section_name.starts_with("Effect.") {
                skipping = true;
                n_removed_effects += 1;
            } else if reset_movements && section_name.starts_with("Movement.") {
                skipping = true;
                n_removed_movements += 1;
            }
        }

        if !skipping {
            writeln!(new_aviutl2_ini_file, "{}", line)?;
        }
    }

    fs::copy(new_aviutl2_ini.path(), &aviutl2_ini_path).with_context(|| {
        format!(
            "Failed to overwrite aviutl2.ini at {}",
            aviutl2_ini_path.display()
        )
    })?;

    tracing::info!("aviutl2.ini をリセットしました",);

    if reset_effects {
        tracing::info!(
            "削除されたカスタムオブジェクト・エフェクト設定の数: {}",
            n_removed_effects
        );
    }
    if reset_movements {
        tracing::info!("削除された移動方法設定の数: {}", n_removed_movements);
    }

    Ok(())
}
