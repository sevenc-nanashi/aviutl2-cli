use anyhow::{Result, bail};
use fs_err as fs;
use std::path::PathBuf;
use strum::IntoEnumIterator;

#[derive(Debug, Clone)]
struct InitConfig {
    project_id: String,
    project_name: String,
    project_type: ProjectType,
    i18n: bool,
}

#[derive(Debug, Clone, Copy)]
enum ProjectType {
    PluginCpp { plugin_type: PluginType },
    PluginRust { plugin_type: PluginType },
    Script { script_type: ScriptType },
    None,
}

#[derive(Debug, Clone, Copy, strum::EnumString, strum::Display, strum::EnumIter, PartialEq, Eq)]
enum PluginType {
    #[strum(serialize = "入力プラグイン")]
    Input,
    #[strum(serialize = "出力プラグイン")]
    Output,
    #[strum(serialize = "フィルタプラグイン")]
    Filter,
    #[strum(serialize = "スクリプトモジュール")]
    ScriptModule,
    #[strum(serialize = "汎用プラグイン")]
    Common,
}
impl PluginType {
    pub fn suffix(&self) -> &'static str {
        match self {
            PluginType::Input => "aui2",
            PluginType::Output => "auo2",
            PluginType::Filter => "auf2",
            PluginType::ScriptModule => "mod2",
            PluginType::Common => "aux2",
        }
    }
}

#[derive(Debug, Clone, Copy, strum::EnumString, strum::Display, strum::EnumIter)]
enum ScriptType {
    #[strum(serialize = "アニメーション効果")]
    Anm,
    #[strum(serialize = "カスタムオブジェクト")]
    Obj,
    #[strum(serialize = "カメラ効果")]
    Cam,
    #[strum(serialize = "シーンチェンジ")]
    Scn,
    #[strum(serialize = "トラックバー移動方法")]
    Tra,
}
impl ScriptType {
    pub fn suffix(&self) -> &'static str {
        match self {
            ScriptType::Anm => "anm2",
            ScriptType::Obj => "obj2",
            ScriptType::Cam => "cam2",
            ScriptType::Scn => "scn2",
            ScriptType::Tra => "tra2",
        }
    }
}

pub fn run(force: bool, use_pkl: bool, _opts: &crate::config::ConfigLoadOpts) -> Result<()> {
    let filename = if use_pkl {
        "aviutl2.pkl"
    } else {
        "aviutl2.toml"
    };
    let path = PathBuf::from(filename);
    if path.exists() {
        if force {
            tracing::warn!("{filename} は既に存在します。上書きします。");
        } else {
            bail!("{filename} は既に存在します。上書きする場合は --force を指定してください。",);
        }
    }
    let current_dir = std::env::current_dir()?;

    let init_config = if dialoguer::console::user_attended() {
        ask_init_config(&current_dir)?
    } else {
        let project_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my_aviutl2_project")
            .to_string();
        let project_id = slugify(&project_name);
        InitConfig {
            project_id,
            project_name,
            project_type: ProjectType::PluginCpp {
                plugin_type: PluginType::Common,
            },
            i18n: false,
        }
    };
    let template = if use_pkl {
        init_pkl_template(&init_config)
    } else {
        init_toml_template(&init_config)
    };
    fs::write(&path, template)?;
    tracing::info!("{filename} を作成しました");

    let gitignore_path = PathBuf::from(".gitignore");
    if gitignore_path.exists() {
        let mut content = fs::read_to_string(&gitignore_path)?;
        content.push_str("\n# AviUtl2 CLI\n/.aviutl2-cli\n/release\n");
        fs::write(&gitignore_path, content)?;
        tracing::info!(".gitignore を更新しました");
    } else {
        fs::write(&gitignore_path, "# AviUtl2 CLI\n/.aviutl2-cli\n/release\n")?;
        tracing::info!(".gitignore を作成しました");
    }
    Ok(())
}

fn ask_init_config(current_dir: &std::path::Path) -> Result<InitConfig> {
    let current_dir_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my_aviutl2_project");
    let project_name = dialoguer::Input::new()
        .with_prompt("プロジェクト名を入力してください")
        .default(current_dir_name.to_string())
        .interact_text()?;
    let project_id = dialoguer::Input::new()
        .with_prompt("プロジェクトIDを入力してください")
        .default(slugify(&project_name))
        .interact_text()?;

    let project_type = dialoguer::Select::new()
        .with_prompt("プロジェクトの種類を選択してください")
        .items([
            "プラグイン（C++）",
            "プラグイン（Rust）",
            "スクリプト",
            "テンプレートなし",
        ])
        .interact()?;

    let project_type = match project_type {
        0 | 1 => {
            let plugin_type = dialoguer::Select::new()
                .with_prompt("プラグインの種類を選択してください")
                .items(
                    PluginType::iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>(),
                )
                .interact()?;
            let plugin_type = PluginType::iter().nth(plugin_type).unwrap();
            if project_type == 0 {
                ProjectType::PluginCpp { plugin_type }
            } else {
                ProjectType::PluginRust { plugin_type }
            }
        }
        2 => {
            let script_type = dialoguer::Select::new()
                .with_prompt("スクリプトの種類を選択してください")
                .items(
                    ScriptType::iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>(),
                )
                .interact()?;
            let script_type = ScriptType::iter().nth(script_type).unwrap();
            ProjectType::Script { script_type }
        }
        3 => ProjectType::None,
        _ => unreachable!(),
    };

    let i18n = !matches!(project_type, ProjectType::None)
        && dialoguer::Confirm::new()
            .with_prompt("英語対応を追加しますか？")
            .interact()?;

    Ok(InitConfig {
        project_id,
        project_name,
        project_type,
        i18n,
    })
}

fn init_toml_template(config: &InitConfig) -> String {
    let mut template = format!(
        dedent::dedent!(
            r#"
            #:schema ./.aviutl2-cli/aviutl2.schema.json
            # 設定ファイルについては https://github.com/sevenc-nanashi/aviutl2-cli を参照してください。
            [project]
            id = "{project_id}"
            name = "{project_name}"
            version = "0.1.0"

            [development]
            aviutl2_version = "latest"
            "#
        ),
        project_id = config.project_id,
        project_name = config.project_name
    );
    template.push('\n');

    let project_filename = slugify(config.project_id.split(".").last().unwrap());

    if config.i18n {
        template.push('\n');
        template.push_str(&format!(
            dedent::dedent!(
                r#"
                [artifacts.English-{project_filename}-aul2]
                destination = "Language/English.{project_filename}.aul2"
                source = "./i18n/English.{project_filename}.aul2"

                [artifacts.English-aul2]
                enabled = false
                destination = "Language/English.aul2"
                source = "https://raw.githubusercontent.com/aviutl2/aviutl2_community_translation/refs/heads/main/locales/original_english.aul2"

                [artifacts.English-aul2.profiles.debug]
                enabled = true
                "#
            ),
            project_filename = project_filename
        ));
        template.push('\n');
    }

    match config.project_type {
        ProjectType::PluginCpp { plugin_type } => {
            let suffix = plugin_type.suffix();
            let target_dir = if plugin_type == PluginType::ScriptModule {
                "Script"
            } else {
                "Plugin"
            };
            template.push('\n');
            template.push_str(&format!(
                dedent::dedent!(
                    r#"
                    [artifacts.{project_filename}-{suffix}]
                    destination = "{target_dir}/{project_filename}.{suffix}"

                    [artifacts.{project_filename}-{suffix}.profiles.debug]
                    build = ["cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug", "cmake --build build --config Debug"]
                    source = "build/Debug/{project_filename}.dll"

                    [artifacts.{project_filename}-{suffix}.profiles.release]
                    build = ["cmake -S . -B build -DCMAKE_BUILD_TYPE=Release", "cmake --build build --config Release"]
                    source = "build/Release/{project_filename}.dll"
                    "#
                ),
                target_dir = target_dir,
                suffix = suffix,
                project_filename = project_filename
            ));
            template.push('\n');
        }
        ProjectType::PluginRust { plugin_type } => {
            let suffix = plugin_type.suffix();
            let target_dir = if plugin_type == PluginType::ScriptModule {
                "Script"
            } else {
                "Plugin"
            };
            template.push('\n');
            template.push_str(&format!(
                dedent::dedent!(
                    r#"
                    [artifacts.{project_filename}-{suffix}]
                    destination = "{target_dir}/{project_filename}.{suffix}"

                    [artifacts.{project_filename}-{suffix}.profiles.debug]
                    build = "cargo build"
                    source = "target/debug/{project_filename}.dll"

                    [artifacts.{project_filename}-{suffix}.profiles.release]
                    build = "cargo build --release"
                    source = "target/release/{project_filename}.dll"
                    "#
                ),
                target_dir = target_dir,
                project_filename = project_filename,
                suffix = suffix
            ));
            template.push('\n');
        }
        ProjectType::Script { script_type } => {
            let suffix = script_type.suffix();
            template.push('\n');
            template.push_str(&format!(
                dedent::dedent!(
                    r#"
                    [artifacts.{project_filename}-{suffix}]
                    destination = "Script/{project_filename}.{suffix}"
                    source = "src/{project_filename}.lua"
                    "#
                ),
                project_filename = project_filename,
                suffix = suffix
            ));
            template.push('\n');
        }
        ProjectType::None => {}
    }

    template.trim_end_matches('\n').to_string()
}

fn init_pkl_template(config: &InitConfig) -> String {
    let mut template = format!(
        dedent::dedent!(
            r#"
            // 設定ファイルについては https://github.com/sevenc-nanashi/aviutl2-cli を参照してください。
            amends "https://raw.githubusercontent.com/sevenc-nanashi/aviutl2-cli/refs/tags/{}/src/AviUtl2CliConfig.pkl"

            project {{
              id = {}
              name = {}
              version = "0.1.0"
            }}

            development {{
              aviutl2_version = "latest"
            }}
            "#
        ),
        env!("CARGO_PKG_VERSION"),
        pkl_string(&config.project_id),
        pkl_string(&config.project_name)
    );

    template.push('\n');

    let project_filename = slugify(config.project_id.split(".").last().unwrap());
    let mut artifacts = Vec::new();

    if config.i18n {
        artifacts.push(format!(
            dedent::dedent!(
                r#"
                ["English-{project_filename}-aul2"] {{
                  destination = "Language/English.{project_filename}.aul2"
                  source = "./i18n/English.{project_filename}.aul2"
                }}

                ["English-aul2"] {{
                  enabled = false
                  destination = "Language/English.aul2"
                  source = "https://raw.githubusercontent.com/aviutl2/aviutl2_community_translation/refs/heads/main/locales/original_english.aul2"
                  profiles {{
                    ["debug"] {{
                      enabled = true
                    }}
                  }}
                }}
                "#
            ),
            project_filename = project_filename
        ));
    }

    match config.project_type {
        ProjectType::PluginCpp { plugin_type } => {
            let suffix = plugin_type.suffix();
            artifacts.push(format!(
                dedent::dedent!(
                    r#"
                    ["{project_filename}-{suffix}"] {{
                      destination = "Plugin/{project_filename}.{suffix}"
                      profiles {{
                        ["debug"] {{
                          build = List("cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug", "cmake --build build --config Debug")
                          source = "build/Debug/{project_filename}.dll"
                        }}
                        ["release"] {{
                          build = List("cmake -S . -B build -DCMAKE_BUILD_TYPE=Release", "cmake --build build --config Release")
                          source = "build/Release/{project_filename}.dll"
                        }}
                      }}
                    }}
                    "#
                ),
                suffix = suffix,
                project_filename = project_filename
            ));
        }
        ProjectType::PluginRust { plugin_type } => {
            let suffix = plugin_type.suffix();
            artifacts.push(format!(
                dedent::dedent!(
                    r#"
                    ["{project_filename}-{suffix}"] {{
                      destination = "Plugin/{project_filename}.{suffix}"
                      profiles {{
                        ["debug"] {{
                          build = "cargo build"
                          source = "target/debug/{project_filename}.dll"
                        }}
                        ["release"] {{
                          build = "cargo build --release"
                          source = "target/release/{project_filename}.dll"
                        }}
                      }}
                    }}
                    "#
                ),
                project_filename = project_filename,
                suffix = suffix
            ));
        }
        ProjectType::Script { script_type } => {
            let suffix = script_type.suffix();
            artifacts.push(format!(
                dedent::dedent!(
                    r#"
                    ["{project_filename}-{suffix}"] {{
                      destination = "Script/{project_filename}.{suffix}"
                      source = "src/{project_filename}.lua"
                    }}
                    "#
                ),
                project_filename = project_filename,
                suffix = suffix
            ));
        }
        ProjectType::None => {}
    }

    if !artifacts.is_empty() {
        template.push('\n');
        template.push_str("artifacts {\n");
        for artifact in artifacts {
            template.push_str(&indent_pkl_block(&artifact));
            template.push('\n');
        }
        template.push('}');
        template.push('\n');
    }

    template.trim_end_matches('\n').to_string()
}

fn pkl_string(value: &str) -> String {
    format!("{value:?}")
}

fn indent_pkl_block(value: &str) -> String {
    value
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn slugify(s: &str) -> String {
    let mut slug = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('_') {
            slug.push('_');
        }
    }
    slug.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("My Project"), "my_project");
        assert_eq!(slugify("Hello, World!"), "hello_world");
        assert_eq!(slugify("AviUtl2 Plugin"), "aviutl2_plugin");
        assert_eq!(slugify("  Leading and trailing  "), "leading_and_trailing");
        assert_eq!(slugify("Multiple   Spaces"), "multiple_spaces");
        assert_eq!(
            slugify("Special-Characters!@#$%^&*()"),
            "special_characters"
        );
        assert_eq!(slugify("Already_Slugified"), "already_slugified");
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn test_init_template() {
        let config = InitConfig {
            project_id: "my_plugin".to_string(),
            project_name: "My Plugin".to_string(),
            project_type: ProjectType::PluginCpp {
                plugin_type: PluginType::Filter,
            },
            i18n: true,
        };
        let template = init_toml_template(&config);
        insta::assert_snapshot!(template);
    }

    #[test]
    fn test_init_pkl_template() {
        let config = InitConfig {
            project_id: "my_plugin".to_string(),
            project_name: "My Plugin".to_string(),
            project_type: ProjectType::PluginCpp {
                plugin_type: PluginType::Filter,
            },
            i18n: true,
        };
        let template = init_pkl_template(&config).replace(env!("CARGO_PKG_VERSION"), "{{VERSION}}");
        insta::assert_snapshot!(template);
    }
}
