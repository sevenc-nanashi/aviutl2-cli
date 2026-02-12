import fs from "node:fs";
process.chdir(import.meta.dirname);

const base = JSON.parse(
  await fs.promises.readFile("temporary/aviutl2.config.schema.json", "utf-8"),
);
const modified = base["$defs"]["Config"];
modified["$id"] = "aviutl2.config.schema.json";

for (const [key, value] of Object.entries(modified["$defs"])) {
  if (key.startsWith("Record")) {
    const v = value as Record<string, any>;
    v["additionalProperties"] = v["unevaluatedProperties"];
    delete v["unevaluatedProperties"];
  }
}

const buildGroup = modified["properties"]["build_group"];
buildGroup["additionalProperties"] = buildGroup["unevaluatedProperties"];
delete buildGroup["unevaluatedProperties"];

// await Bun.write("../src/schema.json", JSON.stringify(modified, null, 2));
await fs.promises.writeFile(
  "../src/schema.json",
  JSON.stringify(modified, null, 2),
);

export {};
