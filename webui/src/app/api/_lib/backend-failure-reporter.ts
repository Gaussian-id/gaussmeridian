import { logger } from "@/utils/logger";

const CAPTURE_LIMIT_BYTES = 16 * 1024;
const CAPTURE_TIMEOUT_MS = 250;
const ERROR_FIELD_LIMIT = 512;
const REQUEST_ID_LIMIT = 256;

const SENSITIVE_ASSIGNMENT =
  /\b(password|passcode|token|access[_-]?token|refresh[_-]?token|api[_ -]?key|idempotency[_ -]?key|cookie|secret)\b\s*[:=]\s*(?:"[^"]*"|'[^']*'|[^\s,;}\]]+)/gi;
const AUTHORIZATION_CREDENTIAL =
  /\bauthorization\b\s*[:=]\s*(?:"[^"]*"|'[^']*'|(?:Basic|Bearer|Digest)\s+[^\s,;}\]]+|[^\s,;}\]]+)/gi;
const BEARER_CREDENTIAL = /\bBearer\s+[^\s,;}\]]+/gi;
const JWT_CREDENTIAL = /\b[A-Za-z0-9_-]{6,}\.[A-Za-z0-9_-]{6,}\.[A-Za-z0-9_-]{6,}\b/g;
const API_KEY_CREDENTIAL = /\b(?:sk|pk|rk)[-_][A-Za-z0-9_-]{8,}\b/gi;
const GAUSSMERIDIAN_API_KEY_CREDENTIAL = /\bgrk_(?:live|test)_[A-Za-z0-9_-]{8,}\b/gi;

type BodyState =
  | "structured"
  | "empty"
  | "malformed"
  | "unsupported"
  | "truncated"
  | "timeout"
  | "unavailable"
  | "unrecognized";

type ReadResult =
  | { state: "captured"; text: string }
  | { state: "truncated" | "timeout" | "unavailable" };

interface SafeBackendError {
  message?: string;
  type?: string;
  code?: string;
  param?: string;
  request_id?: string;
}

interface InspectionResult {
  state: BodyState;
  error?: SafeBackendError;
  requestId?: string;
}

export interface BackendFailureContext {
  method: string;
  path: string;
  phase: string;
}

function cancelReader(reader: ReadableStreamDefaultReader<Uint8Array>): void {
  try {
    void reader.cancel().catch(() => undefined);
  } catch {
    // Diagnostic cancellation must never replace the original backend response.
  }
}

function boundAndRedact(value: unknown, limit = ERROR_FIELD_LIMIT): string | undefined {
  if (typeof value !== "string") return undefined;

  const redacted = value
    .replace(SENSITIVE_ASSIGNMENT, "$1=[REDACTED]")
    .replace(AUTHORIZATION_CREDENTIAL, "authorization=[REDACTED]")
    .replace(BEARER_CREDENTIAL, "Bearer [REDACTED]")
    .replace(JWT_CREDENTIAL, "[REDACTED]")
    .replace(GAUSSMERIDIAN_API_KEY_CREDENTIAL, "[REDACTED]")
    .replace(API_KEY_CREDENTIAL, "[REDACTED]");

  return redacted.length > limit ? `${redacted.slice(0, limit)}…` : redacted;
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : undefined;
}

function sanitizePath(path: string): string {
  const delimiter = path.search(/[?#]/);
  const pathOnly = delimiter === -1 ? path : path.slice(0, delimiter);
  return boundAndRedact(pathOnly, 1024) ?? "";
}

function isJsonContentType(contentType: string): boolean {
  const mimeType = contentType.split(";", 1)[0].trim().toLowerCase();
  return mimeType === "application/json" || mimeType.endsWith("+json");
}

function mergeChunks(chunks: Uint8Array[], byteLength: number): string {
  const captured = new Uint8Array(byteLength);
  let offset = 0;
  for (const chunk of chunks) {
    captured.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder().decode(captured);
}

async function readClone(reader: ReadableStreamDefaultReader<Uint8Array>): Promise<ReadResult> {
  const chunks: Uint8Array[] = [];
  let byteLength = 0;

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) return { state: "captured", text: mergeChunks(chunks, byteLength) };
      if (!value) continue;
      if (byteLength + value.byteLength > CAPTURE_LIMIT_BYTES) {
        cancelReader(reader);
        return { state: "truncated" };
      }
      chunks.push(value);
      byteLength += value.byteLength;
    }
  } catch {
    return { state: "unavailable" };
  }
}

async function readWithTimeout(response: Response): Promise<ReadResult> {
  let clone: Response;
  try {
    clone = response.clone();
  } catch {
    return { state: "unavailable" };
  }

  if (!clone.body) return { state: "captured", text: "" };

  const reader = clone.body.getReader();
  let timeout: ReturnType<typeof setTimeout> | undefined;
  const timeoutResult = new Promise<ReadResult>((resolve) => {
    timeout = setTimeout(() => {
      cancelReader(reader);
      resolve({ state: "timeout" });
    }, CAPTURE_TIMEOUT_MS);
  });

  const result = await Promise.race([readClone(reader), timeoutResult]);
  if (timeout !== undefined) clearTimeout(timeout);
  return result;
}

function extractSafeError(value: unknown): SafeBackendError | undefined {
  const root = asRecord(value);
  const error = asRecord(root?.error);
  if (!error) return undefined;

  const safeError: SafeBackendError = {};
  for (const key of ["message", "type", "code", "param", "request_id"] as const) {
    const safeValue = boundAndRedact(
      error[key],
      key === "request_id" ? REQUEST_ID_LIMIT : ERROR_FIELD_LIMIT,
    );
    if (safeValue !== undefined) safeError[key] = safeValue;
  }

  return Object.keys(safeError).length > 0 ? safeError : undefined;
}

function extractRootRequestId(value: unknown): string | undefined {
  return boundAndRedact(asRecord(value)?.request_id, REQUEST_ID_LIMIT);
}

async function inspectFailure(response: Response, contentType: string): Promise<InspectionResult> {
  const contentLengthHeader = response.headers.get("content-length");
  const contentLength = contentLengthHeader === null ? undefined : Number(contentLengthHeader);
  if (!response.body || contentLength === 0) return { state: "empty" };
  if (!isJsonContentType(contentType)) return { state: "unsupported" };
  if (
    contentLength !== undefined &&
    Number.isFinite(contentLength) &&
    contentLength > CAPTURE_LIMIT_BYTES
  ) {
    return { state: "truncated" };
  }

  const readResult = await readWithTimeout(response);
  if (readResult.state !== "captured") return { state: readResult.state };
  if (readResult.text.trim().length === 0) return { state: "empty" };

  let parsed: unknown;
  try {
    parsed = JSON.parse(readResult.text);
  } catch {
    return { state: "malformed" };
  }

  const error = extractSafeError(parsed);
  const requestId = error?.request_id ?? extractRootRequestId(parsed);
  return error ? { state: "structured", error, requestId } : { state: "unrecognized", requestId };
}

/**
 * Endpoints the console probes to discover whether anyone is signed in. A 401 from these is the
 * expected answer for a signed-out visitor, not a backend failure — on a fresh `docker compose up`
 * the very first page load produces one, and logging it at error level makes a healthy stack look
 * broken. Any OTHER status from these paths is still reported normally, and a 401 from any other
 * path is still an error.
 */
const SESSION_PROBE_PATHS = new Set(["v1/auth/me", "/v1/auth/me", "auth/me"]);

function isExpectedSignedOutProbe(status: number, path: string): boolean {
  return status === 401 && SESSION_PROBE_PATHS.has(path.replace(/^\/+/, ""));
}

export async function reportBackendFailure(
  response: Response,
  context: BackendFailureContext,
): Promise<void> {
  if (response.ok) return;

  const rawContentType = response.headers.get("content-type") ?? "";
  let inspection: InspectionResult;
  try {
    inspection = await inspectFailure(response, rawContentType);
  } catch {
    inspection = { state: "unavailable" };
  }

  const headerRequestId = boundAndRedact(response.headers.get("x-request-id"), REQUEST_ID_LIMIT);
  const requestId = inspection.requestId ?? headerRequestId;
  const event = {
    method: boundAndRedact(context.method, 16) ?? "",
    path: sanitizePath(context.path),
    phase: boundAndRedact(context.phase, 64) ?? "",
    status: response.status,
    statusText: boundAndRedact(response.statusText, 256) ?? "",
    contentType: boundAndRedact(rawContentType, 256) ?? "",
    ...(requestId ? { request_id: requestId } : {}),
    body: { state: inspection.state },
    ...(inspection.error ? { error: inspection.error } : {}),
  };

  try {
    const expected = isExpectedSignedOutProbe(response.status, context.path);
    logger(expected ? "session-probe" : "backend-failure", event, expected ? "info" : "error");
  } catch {
    // Failure reporting must remain observational and never alter the response path.
  }
}
