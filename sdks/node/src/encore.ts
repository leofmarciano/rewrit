import {
  addEffect,
  canonicalValue,
  caseDiscovered,
  observe,
  observeCanonical,
  runtimeId,
  type CanonicalValue,
  type Effect,
} from "./index";

export function encoreCase(caseId: string, suiteId?: string, title?: string) {
  caseDiscovered(caseId, suiteId, title);
}

export function observeServiceResult(value: unknown, caseId?: string, effects: Effect[] = []) {
  observe(value, caseId, effects);
}

export async function observeHttpResponse(response: unknown, caseId?: string, effects: Effect[] = []) {
  const status = statusCode(response);
  observeCanonical(
    {
      kind: "object",
      fields: {
        status: { kind: "integer", value: String(status) },
        headers: { kind: "object", fields: canonicalHeaders(headers(response)) },
        body: await body(response),
      },
    },
    caseId,
    status < 500 ? "passed" : "failed",
    effects,
  );
}

export function observeDbDelta(
  table: string,
  rows: { inserted?: Record<string, unknown>[]; updated?: Record<string, unknown>[]; deleted?: Record<string, unknown>[] },
  connection = "default",
  caseId?: string,
) {
  addEffect(dbDelta(table, rows, connection), caseId);
}

export function dbDelta(
  table: string,
  rows: { inserted?: Record<string, unknown>[]; updated?: Record<string, unknown>[]; deleted?: Record<string, unknown>[] },
  connection = "default",
): Effect {
  return {
    kind: "db_delta",
    connection,
    table,
    inserted: (rows.inserted ?? []).map(canonicalRow),
    updated: (rows.updated ?? []).map(canonicalRow),
    deleted: (rows.deleted ?? []).map(canonicalRow),
  };
}

export function encoreRuntimeMetadata() {
  return {
    runtime_id: runtimeId(),
    encore_app_id: process.env.ENCORE_APP_ID,
    encore_environment: process.env.ENCORE_ENVIRONMENT ?? process.env.ENCORE_RUNTIME_ENV,
  };
}

function statusCode(response: unknown) {
  const candidate = response as { status?: unknown; statusCode?: unknown };
  return Number(candidate.status ?? candidate.statusCode ?? 0);
}

function headers(response: unknown): Record<string, string> {
  const candidate = response as { headers?: unknown };
  const source = candidate.headers;
  if (!source) return {};

  const iterableHeaders = source as { forEach?: (callback: (value: string, key: string) => void) => void };
  if (typeof iterableHeaders.forEach === "function") {
    const entries: Record<string, string> = {};
    iterableHeaders.forEach((value, key) => {
      entries[key.toLowerCase()] = value;
    });
    return entries;
  }

  return Object.fromEntries(
    Object.entries(source as Record<string, unknown>).map(([key, value]) => [key.toLowerCase(), String(value)]),
  );
}

function canonicalHeaders(values: Record<string, string>): Record<string, CanonicalValue> {
  return Object.fromEntries(Object.entries(values).map(([key, value]) => [key, { kind: "string", value }]));
}

async function body(response: unknown): Promise<CanonicalValue> {
  const candidate = response as { json?: () => Promise<unknown> | unknown; text?: () => Promise<string> | string; body?: unknown };
  if (typeof candidate.json === "function") {
    try {
      return { kind: "json", value: await candidate.json() };
    } catch {
      // Fall through to text/body handling.
    }
  }
  if (typeof candidate.text === "function") {
    const text = await candidate.text();
    try {
      return { kind: "json", value: JSON.parse(text) };
    } catch {
      return { kind: "string", value: text };
    }
  }
  return canonicalValue(candidate.body);
}

function canonicalRow(row: Record<string, unknown>): Record<string, CanonicalValue> {
  return Object.fromEntries(Object.entries(row).map(([field, value]) => [field, canonicalValue(value)]));
}

export { addEffect, canonicalValue, observe, observeCanonical };
