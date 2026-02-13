use anyhow::{Result, bail};
use fs_err as fs;
use std::path::PathBuf;

const INIT_TEMPLATE: &str = r#"#:schema ./.aviutl2-cli/aviutl2.schema.json
# 設定ファイルについては https://github.com/sevenc-nanashi/aviutl2-cli を参照してください。
[project]
name = "{{project_name}}"
version = "0.1.0"

[artifacts.my_plugin_aux2]
enabled = true
destination = "Plugin/my_plugin.aux2"

[artifacts.my_plugin_aux2.profiles.debug]
build = "cargo build"
source = "target/debug/my_plugin_aux2.dll"
enabled = true

[artifacts.my_plugin_aux2.profiles.release]
build = ["cargo build --release"]
source = "target/release/my_plugin_aux2.dll"
enabled = true

[development]
aviutl2_version = "latest"

[release]
package_template = "package_template.txt"
"#;

pub fn run() -> Result<()> {
    let path = PathBuf::from("aviutl2.toml");
    if path.exists() {
        bail!("aviutl2.toml は既に存在します");
    }
    let current_dir = std::env::current_dir()?;
    let project_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my_aviutl2_project");
    fs::write(
        &path,
        INIT_TEMPLATE.replace("{{project_name}}", project_name),
    )?;
    log::info!("aviutl2.toml を作成しました");

    let gitignore_path = PathBuf::from(".gitignore");
    if gitignore_path.exists() {
        let mut content = fs::read_to_string(&gitignore_path)?;
        content.push_str("\n# AviUtl2 CLI\n/.aviutl2-cli\n/release\n");
        fs::write(&gitignore_path, content)?;
        log::info!(".gitignore を更新しました");
    } else {
        fs::write(&gitignore_path, "# AviUtl2 CLI\n/.aviutl2-cli\n/release\n")?;
        log::info!(".gitignore を作成しました");
    }
    Ok(())
}
