/**
 * Small JSON Schema validator for the command-model contract.
 *
 * The repository deliberately does not make a third-party validator a
 * bootstrap dependency.  This covers the draft-2020-12 vocabulary used by
 * the checked-in command model and is intentionally strict about the
 * keywords which affect its wire shape.
 */

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function typeMatches(value, type) {
  switch (type) {
    case "array":
      return Array.isArray(value);
    case "boolean":
      return typeof value === "boolean";
    case "integer":
      return typeof value === "number" && Number.isInteger(value);
    case "null":
      return value === null;
    case "number":
      return typeof value === "number" && Number.isFinite(value);
    case "object":
      return isObject(value);
    case "string":
      return typeof value === "string";
    default:
      return true;
  }
}

function formatIsValid(value, format) {
  if (format === "uri") {
    if (typeof value !== "string") return true;
    try {
      const url = new URL(value);
      return url.protocol.length > 1;
    } catch {
      return false;
    }
  }

  if (format === "date-time") {
    return (
      typeof value === "string" &&
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value) &&
      !Number.isNaN(Date.parse(value))
    );
  }

  return true;
}

function decodePointerPart(part) {
  return part.replaceAll("~1", "/").replaceAll("~0", "~");
}

function resolveReference(rootSchema, reference) {
  if (!reference.startsWith("#/")) {
    throw new Error(`unsupported schema reference: ${reference}`);
  }

  return reference
    .slice(2)
    .split("/")
    .map(decodePointerPart)
    .reduce((current, part) => current?.[part], rootSchema);
}

function pathFor(path, property) {
  return `${path}/${String(property).replaceAll("~", "~0").replaceAll("/", "~1")}`;
}

function validateValue(value, schema, rootSchema, path, errors) {
  if (!schema || typeof schema !== "object") return;

  if (schema.$ref) {
    const target = resolveReference(rootSchema, schema.$ref);
    if (!target) {
      errors.push(`${path}: unresolved schema reference ${schema.$ref}`);
      return;
    }
    validateValue(value, target, rootSchema, path, errors);
    return;
  }

  if (schema.allOf) {
    for (const branch of schema.allOf) {
      validateValue(value, branch, rootSchema, path, errors);
    }
  }

  if (schema.anyOf || schema.oneOf) {
    const keyword = schema.oneOf ? "oneOf" : "anyOf";
    const branches = schema[keyword];
    const successful = [];
    const branchErrors = [];

    for (const branch of branches) {
      const candidateErrors = [];
      validateValue(value, branch, rootSchema, path, candidateErrors);
      if (candidateErrors.length === 0) {
        successful.push(branch);
      } else {
        branchErrors.push(candidateErrors);
      }
    }

    const valid = keyword === "oneOf" ? successful.length === 1 : successful.length > 0;
    if (!valid) {
      const expected = keyword === "oneOf" ? "exactly one" : "at least one";
      errors.push(`${path}: ${keyword} must match ${expected} schema`);
      if (successful.length > 1 && keyword === "oneOf") {
        errors.push(`${path}: ${keyword} matched multiple schemas`);
      } else if (branchErrors.length > 0 && branchErrors[0].length > 0) {
        for (const branchError of branchErrors.flat().slice(0, 8)) {
          errors.push(`${path}: ${branchError}`);
        }
      }
    }
  }

  if (schema.const !== undefined && !Object.is(value, schema.const)) {
    errors.push(`${path}: must equal ${JSON.stringify(schema.const)}`);
  }

  if (schema.enum && !schema.enum.some((allowed) => Object.is(allowed, value))) {
    errors.push(`${path}: must be one of ${schema.enum.map((entry) => JSON.stringify(entry)).join(", ")}`);
  }

  if (schema.type) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!types.some((type) => typeMatches(value, type))) {
      errors.push(`${path}: must be ${types.join(" or ")}`);
      return;
    }
  }

  if (typeof value === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength) {
      errors.push(`${path}: must contain at least ${schema.minLength} characters`);
    }
    if (schema.maxLength !== undefined && value.length > schema.maxLength) {
      errors.push(`${path}: must contain at most ${schema.maxLength} characters`);
    }
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) {
      errors.push(`${path}: does not match ${schema.pattern}`);
    }
    if (schema.format && !formatIsValid(value, schema.format)) {
      errors.push(`${path}: is not a valid ${schema.format}`);
    }
  }

  if (typeof value === "number") {
    if (schema.minimum !== undefined && value < schema.minimum) {
      errors.push(`${path}: must be at least ${schema.minimum}`);
    }
    if (schema.maximum !== undefined && value > schema.maximum) {
      errors.push(`${path}: must be at most ${schema.maximum}`);
    }
  }

  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) {
      errors.push(`${path}: must contain at least ${schema.minItems} items`);
    }
    if (schema.maxItems !== undefined && value.length > schema.maxItems) {
      errors.push(`${path}: must contain at most ${schema.maxItems} items`);
    }
    if (schema.uniqueItems) {
      const serialized = value.map((entry) => JSON.stringify(entry));
      if (new Set(serialized).size !== serialized.length) {
        errors.push(`${path}: must contain unique items`);
      }
    }
    if (schema.items) {
      value.forEach((entry, index) => {
        validateValue(entry, schema.items, rootSchema, pathFor(path, index), errors);
      });
    }
  }

  if (isObject(value)) {
    const properties = schema.properties ?? {};
    const patterns = Object.entries(schema.patternProperties ?? {}).map(
      ([pattern, patternSchema]) => [new RegExp(pattern), patternSchema]
    );

    if (schema.required) {
      for (const required of schema.required) {
        if (!Object.prototype.hasOwnProperty.call(value, required)) {
          errors.push(`${path}: missing required property ${JSON.stringify(required)}`);
        }
      }
    }

    for (const [property, propertySchema] of Object.entries(properties)) {
      if (Object.prototype.hasOwnProperty.call(value, property)) {
        validateValue(value[property], propertySchema, rootSchema, pathFor(path, property), errors);
      }
    }

    for (const [property, propertyValue] of Object.entries(value)) {
      if (Object.prototype.hasOwnProperty.call(properties, property)) continue;
      const matchingPatterns = patterns.filter(([pattern]) => pattern.test(property));
      if (matchingPatterns.length > 0) {
        for (const [, patternSchema] of matchingPatterns) {
          validateValue(propertyValue, patternSchema, rootSchema, pathFor(path, property), errors);
        }
        continue;
      }

      if (schema.additionalProperties === false) {
        errors.push(`${path}: unknown property ${JSON.stringify(property)}`);
      } else if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
        validateValue(
          propertyValue,
          schema.additionalProperties,
          rootSchema,
          pathFor(path, property),
          errors
        );
      }
    }
  }
}

export function validate(instance, schema) {
  const errors = [];
  validateValue(instance, schema, schema, "$", errors);
  return { valid: errors.length === 0, errors };
}

export function assertValid(instance, schema, label = "instance") {
  const result = validate(instance, schema);
  if (!result.valid) {
    throw new Error(`${label} failed schema validation:\n${result.errors.join("\n")}`);
  }
  return instance;
}
