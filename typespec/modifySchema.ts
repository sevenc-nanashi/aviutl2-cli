process.chdir(import.meta.dir);

const base = JSON.parse(
  await Bun.file("temporary/aviutl2.config.schema.json").text(),
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

await Bun.write("../src/schema.json", JSON.stringify(modified, null, 2));

export {};
