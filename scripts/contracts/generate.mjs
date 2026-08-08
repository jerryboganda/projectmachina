import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const schemaPath = join(root, "schemas/command-model/v0.1/command-model.json");
const outputRoot = process.env.MACHINA_CONTRACT_OUTPUT_DIR
  ? resolve(root, process.env.MACHINA_CONTRACT_OUTPUT_DIR)
  : root;
const rustPath = join(outputRoot, "crates/command-model/src/generated.rs");
const typescriptPath = join(outputRoot, "packages/contracts-ts/src/command-model.ts");

const schemaText = (await readFile(schemaPath, "utf8")).replace(/\r\n/g, "\n");
const schema = JSON.parse(schemaText);
const sourceHash = createHash("sha256").update(schemaText).digest("hex");
const codegen = schema["x-machina-codegen"];

if (!codegen || codegen.version !== "0.1.0") {
  throw new Error("command schema is missing a supported x-machina-codegen version");
}

const enumNames = {
  EngineKind: "EngineKind",
  DataClassification: "DataClassification",
  CommandKind: "CommandKind",
  OutcomeStatus: "OutcomeStatus",
  CapabilityStatus: "CapabilityStatus",
  CanonicalErrorCode: "CanonicalErrorCode",
  EventType: "EventType"
};

function rustEnum(name, values) {
  const variants = values.map((value) => {
    const variant = value
      .split(/[-_.]/)
      .filter(Boolean)
      .map((part) => {
        const normalized = part.toLowerCase();
        return normalized[0].toUpperCase() + normalized.slice(1);
      })
      .join("");
    return `    ${variant},`;
  });
  return `#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub enum ${name} {\n${variants.join("\n")}\n}`;
}

function tsEnum(name, values) {
  const entries = values.map((value) => {
    const memberName = value
      .split(/[-_.]/)
      .filter(Boolean)
      .map((part, index) => {
        const normalized = part.toLowerCase();
        return index === 0
          ? normalized
          : normalized[0].toUpperCase() + normalized.slice(1);
      })
      .join("");
    return `  ${memberName} = ${JSON.stringify(value)},`;
  });
  return `export enum ${name} {\n${entries.join("\n")}\n}`;
}

function rustPayloadUnion() {
  const variants = codegen.payload_variants.map(({ name, type }) => `    ${name}(${type}),`);
  return `#[derive(Clone, Debug, Eq, PartialEq)]\npub enum CommandPayload {\n${variants.join("\n")}\n}`;
}

function tsPayloadUnion() {
  return `export type CommandPayload =\n${codegen.payload_variants
    .map(({ type }) => `  | ${type}`)
    .join("\n")};`;
}

function rustStruct(name, fields) {
  const rendered = Object.entries(fields.properties ?? {}).map(([fieldName, field]) => {
    const optional = !(fields.required ?? []).includes(fieldName);
    let type = "String";
    if (field.type === "boolean") type = "bool";
    if (field.type === "integer") type = field.minimum === 1 ? "u64" : "i64";
    if (field.type === "array") type = "Vec<String>";
    if (field.$ref) type = field.$ref.split("/").at(-1);
    if (optional) type = `Option<${type}>`;
    return `    pub ${fieldName}: ${type},`;
  });
  return `#[derive(Clone, Debug, Eq, PartialEq)]\npub struct ${name} {\n${rendered.join("\n")}\n}`;
}

function tsType(field) {
  if (field.type === "boolean") return "boolean";
  if (field.type === "integer") return "number";
  if (field.type === "array") return "string[]";
  if (field.$ref) return field.$ref.split("/").at(-1);
  return "string";
}

function tsInterface(name, fields) {
  if (name === "CommandEnvelope") {
    const baseFields = Object.fromEntries(
      Object.entries(fields.properties ?? {}).filter(
        ([fieldName]) => fieldName !== "kind" && fieldName !== "payload"
      )
    );
    const base = tsInterface("CommandEnvelopeBase", {
      properties: baseFields,
      required: fields.required?.filter(
        (fieldName) => fieldName !== "kind" && fieldName !== "payload"
      )
    });
    const variants = codegen.payload_variants
      .map(({ kind, type }) => {
        const memberName = kind
          .split(/[-_.]/)
          .filter(Boolean)
          .map((part, index) => {
            const normalized = part.toLowerCase();
            return index === 0
              ? normalized
              : normalized[0].toUpperCase() + normalized.slice(1);
          })
          .join("");
        return `  | CommandEnvelopeBase & { kind: CommandKind.${memberName}; payload: ${type} }`;
      })
      .join("\n");
    return `${base}\n\nexport type CommandEnvelope =\n${variants};`;
  }
  const required = new Set(fields.required ?? []);
  const rendered = Object.entries(fields.properties ?? {}).map(([fieldName, field]) => {
    const suffix = required.has(fieldName) ? "" : "?";
    return `  ${fieldName}${suffix}: ${tsType(field)};`;
  });
  return `export interface ${name} {\n${rendered.join("\n")}\n}`;
}

const definitions = schema.$defs;
const rustParts = [
  "// @generated by scripts/contracts/generate.mjs; do not edit.",
  `// source_schema_sha256: ${sourceHash}`,
  `pub const SCHEMA_VERSION: &str = ${JSON.stringify(codegen.version)};`,
  `pub const SOURCE_SCHEMA_SHA256: &str = ${JSON.stringify(sourceHash)};`,
  ...Object.entries(enumNames).map(([schemaName, rustName]) => rustEnum(rustName, definitions[schemaName].enum)),
  ...["CommandMetadata", "SessionCreatePayload", "NavigationGotoPayload", "SemanticQueryPayload", "ClickPayload", "SessionClosePayload", "EngineExecution", "CanonicalError", "CommandOutcome", "EventEnvelope", "CapabilityStatusRecord"].map((name) => rustStruct(name, definitions[name])),
  rustPayloadUnion(),
  rustStruct("CommandEnvelope", definitions.CommandEnvelope)
];

const tsParts = [
  "// @generated by scripts/contracts/generate.mjs; do not edit.",
  `// source_schema_sha256: ${sourceHash}`,
  `export const SCHEMA_VERSION = ${JSON.stringify(codegen.version)} as const;`,
  `export const SOURCE_SCHEMA_SHA256 = ${JSON.stringify(sourceHash)} as const;`,
  ...Object.entries(enumNames).map(([schemaName, tsName]) => tsEnum(tsName, definitions[schemaName].enum)),
  ...["CommandMetadata", "SessionCreatePayload", "NavigationGotoPayload", "SemanticQueryPayload", "ClickPayload", "SessionClosePayload", "EngineExecution", "CanonicalError", "CommandOutcome", "EventEnvelope", "CapabilityStatusRecord"].map((name) => tsInterface(name, definitions[name])),
  tsPayloadUnion(),
  tsInterface("CommandEnvelope", definitions.CommandEnvelope)
];

await mkdir(dirname(rustPath), { recursive: true });
await mkdir(dirname(typescriptPath), { recursive: true });
await writeFile(rustPath, `${rustParts.join("\n\n")}\n`, "utf8");
await writeFile(typescriptPath, `${tsParts.join("\n\n")}\n`, "utf8");
const rustfmt = spawnSync(
  process.platform === "win32" ? "rustfmt.exe" : "rustfmt",
  ["--edition", "2021", rustPath],
  {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "pipe"
  }
);
if (rustfmt.error || rustfmt.status !== 0) {
  throw new Error(
    rustfmt.stderr || rustfmt.error?.message || "rustfmt failed for generated Rust bindings"
  );
}
console.log(`generated command model from ${sourceHash}`);
