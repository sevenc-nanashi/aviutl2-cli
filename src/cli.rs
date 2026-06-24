use clap::Subcommand;

#[derive(clap::Parser)]
#[command(name = "au2", version, about = "AviUtl2 CLI")]
pub struct Cli {
    /// コンフィグの一部をパッチするファイルのパス
    #[arg(short = 'c', long = "config-patch", global = true)]
    pub config_patch: Option<String>,

    /// コンフィグ全体を置き換えるファイルのパス
    #[arg(short = 'C', long = "config-override", global = true)]
    pub config_override: Option<String>,

    /// 色を無効にします
    #[arg(long = "no-color", global = true)]
    pub no_color: bool,

    /// ログのスタイルを指定します
    #[arg(long = "log-style", global = true, default_value = "original")]
    pub log_style: LogStyle,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum LogStyle {
    /// aviutl2-cli独自のフォーマット
    Original,
    /// tracing_subscriberのデフォルト
    Default,
    /// tracing_subscriberのcompact
    Compact,
    /// tracing_subscriberのpretty
    Pretty,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 設定ファイルを初期化します
    Init {
        /// 既存ファイルがある場合に上書きします
        #[arg(short, long)]
        force: bool,
        /// 設定ファイルに pkl を使用します
        #[arg(long = "pkl")]
        use_pkl: bool,
    },

    /// AviUtl2 の開発環境をセットアップします
    /// （prepare:schema -> prepare:aviutl2 -> prepare:artifacts）
    Prepare {
        /// 既存ファイルがある場合に上書きします
        #[arg(short, long)]
        force: bool,

        /// HTTP の成果物キャッシュを再取得します
        #[arg(short, long)]
        refresh: bool,
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
        #[arg(short, long)]
        force: bool,

        /// 使うプロファイル名（デフォルトは debug）
        #[arg(short = 'p', long = "profile")]
        profile: Option<String>,

        /// HTTP の成果物キャッシュを再取得します
        #[arg(short, long)]
        refresh: bool,
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

        /// HTTP の成果物キャッシュを再取得します
        #[arg(short, long)]
        refresh: bool,

        /// AviUtl2に渡す追加のコマンドライン引数
        args: Vec<String>,
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

    /// リリース成果物をプレビュー用ディレクトリに配置します
    Preview {
        /// 使うプロファイル名（デフォルトは release）
        #[arg(short = 'p', long = "profile")]
        profile: Option<String>,

        /// AviUtl2を起動しない
        #[arg(short = 's', long = "skip-start")]
        skip_start: bool,

        /// HTTP の成果物キャッシュを再取得します
        #[arg(short, long)]
        refresh: bool,

        /// AviUtl2に渡す追加のコマンドライン引数
        args: Vec<String>,
    },
}
