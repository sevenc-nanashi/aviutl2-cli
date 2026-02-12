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

  /** ビルドコマンドのグループ定義 */
  build_group?: Record<BuildCommand>;

  /** 開発用の設定 */
  development?: Development;

  /** プレビュー用の設定 */
  preview?: Preview;

  /** リリース用の設定 */
  release?: Release;

  /** AviUtl2 カタログの定義 */
  catalog?: Catalog;
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
alias BuildCommand = string | string[] | BuildGroupRef;

model BuildGroupRef {
  /** 利用する build_group のキー */
  group: string;
}

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

model Catalog {
  /** ID */
  id: string;

  /** パッケージ名 */
  name: string;

  /** 種類 */
  type:
    | "output"
    | "input"
    | "filter"
    | "common"
    | "modification"
    | "script"
    | "language";

  /** 作者名 */
  author: string;

  /** オリジナル作者名 */
  original_author?: string;

  /** ニコニ・コモンズID */
  niconi_commons_id?: string;

  /** 概要 */
  summary: string;

  /** パッケージのサイト */
  homepage: string;

  /** 依存パッケージ（未使用） */
  dependencies?: string[];

  /** タグ */
  tags?: string[];

  /** 説明 */
  description: string | {
    type: "url";

    /** URL */
    url: string;
  } | {
    type: "inline";

    /** Markdown 形式の内容 */
    content: string;
  };

  /** ライセンス情報 */
  license: CatalogLicense;

  /** ダウンロード元 */
  download_source: CatalogDownloadSource;

  /** インストール手順（省略すると自動生成） */
  install_steps?: CatalogAction[];

  /** アンインストール手順（省略すると自動生成） */
  uninstall_steps?: CatalogAction[];
}

model TemplateCatalogLicense {
  /** ライセンスの種類 */
  type: "MIT" | "Apache-2.0" | "BSD-2-Clause" | "BSD-3-Clause";

  /** テンプレートを使用するかどうか */
  template: true;

  /** 著作者 */
  author: string;

  /** 年 */
  year: string;
}

model CustomCatalogLicense {
  /** ライセンスの種類 */
  type: "mit" | "apache-2.0" | "bsd-2-clause" | "bsd-3-clause";

  /** テンプレートを使用するかどうか */
  template: false;

  /** ライセンスの全文 */
  text: string;
}

model CC0License {
  /** ライセンスの種類 */
  type: "cc0";
}

model OtherCatalogLicense {
  /** ライセンスの種類 */
  type: "other";

  /** ライセンスの名前 */
  name?: "custom" | string;

  /** ライセンスの全文 */
  text: string;
}

model UnknownCatalogLicense {
  /** ライセンスの種類 */
  type: "unknown";
}

alias CatalogLicense =
  | TemplateCatalogLicense
  | CustomCatalogLicense
  | CC0License
  | CustomCatalogLicense
  | UnknownCatalogLicense;

alias CatalogDownloadSource =
  | {
      type: "direct";
      url: string;
    }
  | {
      type: "booth";
      url: string;
    }
  | {
      type: "github";
      owner: string;
      repo: string;
      pattern?: string;
    }
  | {
      type: "google_drive";
      id: string;
    };

model CatalogActionDownload {
  action: "download";
}

model CatalogActionExtract {
  action: "extract";
}

model CatalogActionCopy {
  action: "copy";
  from: string;
  to: string;
}

model CatalogActionDelete {
  action: "delete";
  path: string;
}

model CatalogActionRun {
  action: "run";
  path: string;
  args: string[];
  elevate?: boolean;
}

alias CatalogAction =
  | CatalogActionDownload
  | CatalogActionExtract
  | CatalogActionCopy
  | CatalogActionDelete
  | CatalogActionRun;
