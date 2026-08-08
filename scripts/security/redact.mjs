const REDACTION_MARKER = "[REDACTED]";
const SENSITIVE_KEY = /(authorization|cookie|set[-_]?cookie|password|passphrase|secret|token|api[-_]?key|apikey|private[-_]?key|proxy)/i;
const SENSITIVE_QUERY = /([?&](?:authorization|access_token|id_token|token|api[-_]?key|apikey|password|secret|signature|sig|session)=)[^&#\s]*/gi;
const SENSITIVE_HEADER = /(^|\r?\n)(authorization|cookie|set-cookie|proxy-authorization|x-api-key|x-auth-token|x-access-token|x-session-token|x-csrf-token)\s*:\s*[^\r\n]*/gi;
const SAFE_KEYS = new Set([
  "id",
  "task_id",
  "command_id",
  "session_id",
  "correlation_id",
  "causation_id",
  "event_id",
  "sequence",
  "status",
  "category",
  "code",
  "reason_code",
  "classification",
  "engine",
  "engine_version",
  "capability_id",
  "owner",
  "build_id",
  "environment_id",
  "unit",
  "count",
  "success",
  "verified",
  "retryable"
]);

function encodeSecret(secret) {
  return Buffer.from(secret, "utf8").toString("base64");
}

function encodeSecretUrl(secret) {
  return encodeURIComponent(secret);
}

function encodeSecretBase64Url(secret) {
  return encodeSecret(secret).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

export class Redactor {
  #secrets = new Set();

  registerSecret(secret) {
    if (typeof secret !== "string" || secret.length === 0) {
      throw new Error("redaction secrets must be non-empty strings");
    }
    this.#secrets.add(secret);
  }

  redactText(value) {
    if (typeof value !== "string") {
      throw new TypeError("redactText expects a string");
    }

    let redacted = value;
    const secrets = [...this.#secrets].sort((left, right) => right.length - left.length);
    for (const secret of secrets) {
      redacted = redacted.split(secret).join(REDACTION_MARKER);
      redacted = redacted.split(encodeSecret(secret)).join(REDACTION_MARKER);
      redacted = redacted.split(encodeSecretUrl(secret)).join(REDACTION_MARKER);
      redacted = redacted.split(encodeSecretBase64Url(secret)).join(REDACTION_MARKER);
    }
    redacted = redacted.replace(SENSITIVE_HEADER, `$1$2: ${REDACTION_MARKER}`);
    return redacted.replace(SENSITIVE_QUERY, `$1${REDACTION_MARKER}`);
  }

  redactValue(value, key = "") {
    if (SENSITIVE_KEY.test(key)) {
      return REDACTION_MARKER;
    }
    if (typeof value === "string") {
      return key.length === 0 || SAFE_KEYS.has(key)
        ? this.redactText(value)
        : REDACTION_MARKER;
    }
    if (Array.isArray(value)) {
      return value.map((item) => this.redactValue(item));
    }
    if (value && typeof value === "object") {
      return Object.fromEntries(
        Object.entries(value).map(([entryKey, entryValue]) => [
          entryKey,
          this.redactValue(entryValue, entryKey)
        ])
      );
    }
    return value;
  }

  containsRegisteredSecret(value) {
    if (typeof value !== "string") {
      return false;
    }
    return [...this.#secrets].some(
      (secret) => value.includes(secret) || value.includes(encodeSecret(secret))
    );
  }
}

export { REDACTION_MARKER };
