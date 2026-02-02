use clap::Subcommand;

#[derive(clap::Parser)]
#[command(
    name = "au2",
    version,
    about = "AviUtl2 CLI",
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// aviutl2.toml を作成します
    Init,

    /// AviUtl2 の開発環境をセットアップします
    /// （prepare:schema -> prepare:aviutl2 -> prepare:artifacts）
    Prepare {
        /// 既存ファイルがある場合に上書きします
        #[arg(long)]
        force: bool,
    },

    /// 設定ファイルの JSON Schema を開発用ディレクトリに出力します
    #[command(name = "prepare:schema")]
    PrepareSchema,

    /// AviUtl2 本体をダウンロードし、開発用ディレクトリに展開します
    #[command(name = "prepare:aviutl2")]
    PrepareAviUtl2,

    /// 成果物を開発用ディレクトリに配置します
    #[command(name = "prepare:artifacts")]
    PrepareArtifacts {
        /// 既存ファイルがある場合に上書きします
        #[arg(long)]
        force: bool,

        /// 使うプロファイル名（デフォルトは debug）
        #[arg(short = 'p', long = "profile")]
        profile: Option<String>,
    },

    /// 開発用の成果物をビルドし、AviUtl2 に配置します
    #[command(alias = "dev")]
    Develop {
        /// 使うプロファイル名（デフォルトは debug）
        #[arg(short = 'p', long = "profile")]
        profile: Option<String>,

        /// AviUtl2を起動しない
        #[arg(short = 's', long = "skip-start")]
        skip_start: bool,
    },

    /// リリース用のパッケージを作成します
    Release {
        /// 使うプロファイル名（デフォルトは release）
        #[arg(short = 'p', long = "profile")]
        profile: Option<String>,

        /// 使うバージョン（aviutl2.toml の project.version を上書き）
        #[arg(long = "set-version")]
        set_version: Option<String>,
    },
}
