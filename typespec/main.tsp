/**
 * aviutl2.toml の設定スキーマ定義。
 * JSON Schema は `nr typespec` で生成され、`src/schema.json` に反映されます。
 */
import "@typespec/json-schema";

using TypeSpec.JsonSchema;

@jsonSchema
model Config {
  /** プロジェクト設定 */
  project: Project;
  /** 成果物の定義 */
  artifacts: Record<Artifact>;
  /** 開発用の設定 */
  development?: Development;
  /** プレビュー用の設定 */
  preview?: Preview;
  /** リリース用の設定 */
  release?: Release;
}

model Project {
  /** プロジェクト名 */
  name: string;
  /** バージョン */
  version: string;
}

model Artifact {
  /** 成果物の有効/無効（デフォルトは true） */
  enabled?: boolean;
  /** 成果物のパス */
  source?: string;
  /** AviUtl2 の data 配下での配置先 */
  destination: string;
  /** ビルドコマンド */
  build?: BuildCommand;
  /** 配置方法（symlink / copy） */
  placement_method?: PlacementMethod;
  /** プロファイルごとの設定 */
  profiles?: Record<ArtifactProfile>;
}

model ArtifactProfile {
  /** 成果物の有効/無効（デフォルトは true） */
  enabled?: boolean;
  /** 成果物のパス */
  source?: string;
  /** ビルドコマンド */
  build?: BuildCommand;
}

/** 単一または複数のビルドコマンド */
alias BuildCommand = string | string[];

enum PlacementMethod {
  /** シンボリックリンク */
  symlink,
  /** コピー */
  copy,
}

model Development {
  /** AviUtl2 のバージョン */
  aviutl2_version: string;
  /** AviUtl2 のインストール先 */
  install_dir?: string;
  /** 使うプロファイル名（デフォルトは debug） */
  profile?: string;
  /** 事前ビルドコマンド */
  prebuild?: BuildCommand;
  /** 事後ビルドコマンド */
  postbuild?: BuildCommand;
}

model Preview {
  /** AviUtl2 のバージョン（省略時は development.aviutl2_version） */
  aviutl2_version?: string;
  /** AviUtl2 のインストール先 */
  install_dir?: string;
  /** 使うプロファイル名（デフォルトは release） */
  profile?: string;
  /** 含める成果物のリスト（省略時は release.include を使用） */
  include?: string[];
  /** 事前ビルドコマンド */
  prebuild?: BuildCommand;
  /** 事後ビルドコマンド */
  postbuild?: BuildCommand;
}

model Release {
  /** 出力ディレクトリ */
  output_dir?: string;
  /** package.txt のテンプレート */
  package_template?: string;
  /** zip の名前（`.au2pkg.zip` は自動付与） */
  zip_name?: string;
  /** 使うプロファイル名（デフォルトは release） */
  profile?: string;
  /** 含める成果物のリスト（省略時はすべて含める） */
  include?: string[];
  /** 事前ビルドコマンド */
  prebuild?: BuildCommand;
  /** 事後ビルドコマンド */
  postbuild?: BuildCommand;
}
