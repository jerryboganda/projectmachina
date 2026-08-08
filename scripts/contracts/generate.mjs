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

const inlineEnumNames = {
  "SessionCreatePayload.engine_policy": "EnginePolicy",
  "SessionCreatePayload.fidelity_profile": "FidelityProfile",
  "NavigationGotoPayload.wait_until": "WaitUntil"
};

const definitions = schema.$defs;

function rustMemberName(value) {
  return value
    .split(/[-_.]/)
    .filter(Boolean)
    .map((part) => {
      const normalized = part.toLowerCase();
      return normalized[0].toUpperCase() + normalized.slice(1);
    })
    .join("");
}

function tsMemberName(value) {
  return value
    .split(/[-_.]/)
    .filter(Boolean)
    .map((part, index) => {
      const normalized = part.toLowerCase();
      return index === 0
        ? normalized
        : normalized[0].toUpperCase() + normalized.slice(1);
    })
    .join("");
}

function assertWireShape() {
  const commandKinds = new Set(definitions.CommandKind.enum);
  const branches = new Map(
    definitions.CommandEnvelope.oneOf.map((branch) => [
      branch.properties?.kind?.const,
      branch.properties?.payload?.$ref
    ])
  );
  const payloadRefs = new Set(
    definitions.CommandPayload.oneOf.map((branch) => branch.$ref)
  );

  if (codegen.payload_variants.length !== definitions.CommandEnvelope.oneOf.length) {
    throw new Error("codegen payload variants and command wire branches are out of sync");
  }
  if (codegen.payload_variants.length !== payloadRefs.size) {
    throw new Error("codegen payload variants and payload union are out of sync");
  }

  const seenKinds = new Set();
  for (const { kind, type } of codegen.payload_variants) {
    if (seenKinds.has(kind)) {
      throw new Error(`duplicate codegen payload variant ${kind}`);
    }
    seenKinds.add(kind);
    if (!commandKinds.has(kind)) {
      throw new Error(`codegen payload variant ${kind} is not a CommandKind`);
    }
    if (branches.get(kind) !== `#/$defs/${type}`) {
      throw new Error(`command wire shape disagrees for ${kind}`);
    }
    if (!payloadRefs.has(`#/$defs/${type}`)) {
      throw new Error(`payload wire shape disagrees for ${kind}`);
    }
  }
}

function inlineEnumDefinitions() {
  return Object.entries(inlineEnumNames).map(([path, name]) => {
    const [owner, fieldName] = path.split(".");
    const field = definitions[owner]?.properties?.[fieldName];
    if (!field?.enum) {
      throw new Error(`named inline enum ${path} is missing an enum constraint`);
    }
    return { name, values: field.enum };
  });
}

function rustInlineEnumMethods() {
  return inlineEnumDefinitions()
    .map(({ name, values }) => {
      const asString = values
        .map((value) => `            Self::${rustMemberName(value)} => ${JSON.stringify(value)},`)
        .join("\n");
      const parse = values
        .map((value) => `            ${JSON.stringify(value)} => Some(Self::${rustMemberName(value)}),`)
        .join("\n");
      return `impl ${name} {
    pub const fn as_str(self) -> &'static str {
        match self {
${asString}
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
${parse}
            _ => None,
        }
    }
}`;
    })
    .join("\n\n");
}

function rustConstraintAccessors() {
  return Object.entries(inlineEnumNames)
    .map(([path, enumName]) => {
      const [owner, fieldName] = path.split(".");
      const optional = !(definitions[owner].required ?? []).includes(fieldName);
      const expression = optional
        ? `self.${fieldName}.as_deref().and_then(${enumName}::parse)`
        : `${enumName}::parse(&self.${fieldName})`;
      return `impl ${owner} {
    pub fn ${fieldName}_kind(&self) -> Option<${enumName}> {
        ${expression}
    }
}`;
    })
    .join("\n\n");
}

function rustEnum(name, values) {
  const variants = values.map((value) => {
    return `    ${rustMemberName(value)},`;
  });
  return `#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]\npub enum ${name} {\n${variants.join("\n")}\n}`;
}

function tsEnum(name, values) {
  const entries = values.map((value) => {
    return `  ${tsMemberName(value)} = ${JSON.stringify(value)},`;
  });
  return `export enum ${name} {\n${entries.join("\n")}\n}`;
}

function rustPayloadUnion() {
  const variants = codegen.payload_variants.map(({ name, type }) => `    ${name}(${type}),`);
  return `#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]\npub enum CommandPayload {\n${variants.join("\n")}\n}`;
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
    if (field.type === "object") type = "serde_json::Value";
    if (field.$ref) type = field.$ref.split("/").at(-1);
    if (optional) type = `Option<${type}>`;
    return `    pub ${fieldName}: ${type},`;
  });
  return `#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]\npub struct ${name} {\n${rendered.join("\n")}\n}`;
}

function tsType(field) {
  if (field.type === "boolean") return "boolean";
  if (field.type === "integer") return "number";
  if (field.type === "array") {
    if (field.items?.$ref) return `${field.items.$ref.split("/").at(-1)}[]`;
    return "string[]";
  }
  if (field.type === "object") return "Record<string, unknown>";
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
    const namedEnum = inlineEnumNames[`${name}.${fieldName}`];
    const type = namedEnum ?? tsType(field);
    return `  ${fieldName}${suffix}: ${type};`;
  });
  return `export interface ${name} {\n${rendered.join("\n")}\n}`;
}

function rustDiscriminatorHelpers() {
  const matches = codegen.payload_variants
    .map(({ name, kind }) => `            Self::${name}(_) => CommandKind::${rustMemberName(kind)},`)
    .join("\n");
  return `impl CommandPayload {
    pub const fn kind(&self) -> CommandKind {
        match self {
${matches}
        }
    }

    pub fn matches_kind(&self, kind: CommandKind) -> bool {
        self.kind() == kind
    }
}

impl CommandEnvelope {
    pub fn payload_matches_kind(&self) -> bool {
        self.payload.matches_kind(self.kind)
    }
}`;
}

function tsPayloadMapping() {
  const entries = codegen.payload_variants
    .map(({ kind, type }) => `  [CommandKind.${tsMemberName(kind)}]: ${type};`)
    .join("\n");
  return `export interface CommandPayloadByKind {
${entries}
}

export type CommandPayloadFor<Kind extends CommandKind> = CommandPayloadByKind[Kind];`;
}

assertWireShape();
const rustParts = [
  "// @generated by scripts/contracts/generate.mjs; do not edit.",
  `// source_schema_sha256: ${sourceHash}`,
  "use serde::{Deserialize, Serialize};",
  `pub const SCHEMA_VERSION: &str = ${JSON.stringify(codegen.version)};`,
  `pub const SOURCE_SCHEMA_SHA256: &str = ${JSON.stringify(sourceHash)};`,
  ...Object.entries(enumNames).map(([schemaName, rustName]) => rustEnum(rustName, definitions[schemaName].enum)),
  ...inlineEnumDefinitions().map(({ name, values }) => rustEnum(name, values)),
  rustInlineEnumMethods(),
  ...["CommandMetadata", "SessionCreatePayload", "NavigationGotoPayload", "SemanticQueryPayload", "ClickPayload", "SessionClosePayload", "EngineExecution", "CanonicalError", "CommandOutcome", "EventEnvelope", "CapabilityStatusRecord"].map((name) => rustStruct(name, definitions[name])),
  rustConstraintAccessors(),
  rustPayloadUnion(),
  rustStruct("CommandEnvelope", definitions.CommandEnvelope),
  rustDiscriminatorHelpers()
];

const tsParts = [
  "// @generated by scripts/contracts/generate.mjs; do not edit.",
  `// source_schema_sha256: ${sourceHash}`,
  `export const SCHEMA_VERSION = ${JSON.stringify(codegen.version)} as const;`,
  `export const SOURCE_SCHEMA_SHA256 = ${JSON.stringify(sourceHash)} as const;`,
  ...Object.entries(enumNames).map(([schemaName, tsName]) => tsEnum(tsName, definitions[schemaName].enum)),
  ...inlineEnumDefinitions().map(({ name, values }) => tsEnum(name, values)),
  ...["CommandMetadata", "SessionCreatePayload", "NavigationGotoPayload", "SemanticQueryPayload", "ClickPayload", "SessionClosePayload", "EngineExecution", "CanonicalError", "CommandOutcome", "EventEnvelope", "CapabilityStatusRecord"].map((name) => tsInterface(name, definitions[name])),
  tsPayloadUnion(),
  tsPayloadMapping(),
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
