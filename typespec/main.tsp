import "@typespec/json-schema";

using TypeSpec.JsonSchema;

@jsonSchema
model Config {
  project: Project;
  artifacts: Record<Artifact>;
  development?: Development;
  release?: Release;
}

model Project {
  name: string;
  version: string;
}

model Artifact {
  source?: string;
  destination: string;
  build?: BuildCommand;
  placement_method?: PlacementMethod;
  profiles?: Record<ArtifactProfile>;
}

model ArtifactProfile {
  source?: string;
  build?: BuildCommand;
}

alias BuildCommand = string | string[];

enum PlacementMethod {
  symlink,
  copy,
}

model Development {
  aviutl2_version: string;
  install_dir?: string;
  profile?: string;
}

model Release {
  output_dir?: string;
  package_template?: string;
  zip_name?: string;
  profile?: string;
  include?: string[];
}
