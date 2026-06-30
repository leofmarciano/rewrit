import { appendFileSync } from "node:fs";

export type RuntimeId = string;
export type CaseId = string;

export type CanonicalValue =
  | { kind: "null" }
  | { kind: "bool"; value: boolean }
  | { kind: "integer"; value: string }
  | { kind: "float"; value: string }
  | { kind: "string"; value: string }
  | { kind: "array"; items: CanonicalValue[] }
  | { kind: "object"; fields: Record<string, CanonicalValue> }
  | { kind: "json"; value: unknown };

export type Effect = Record<string, unknown> & { kind: string };

let currentCaseId: CaseId | undefined;
let currentSuiteId: string | undefined;
let lastObservation: Record<string, unknown> | undefined;

export function runtimeId(): RuntimeId {
  return process.env.REWRIT_RUNTIME_ID || "candidate";
}

export function caseDiscovered(caseId: CaseId, suiteId = suiteFromCaseId(caseId), title = caseId) {
  currentCaseId = caseId;
  currentSuiteId = suiteId;
  emitEvent({
    schema_version: "rewrit.event.v1",
    kind: "case_discovered",
    runtime_id: runtimeId(),
    case: {
      id: caseId,
      suite_id: suiteId,
      title,
      source_location: null,
      tags: [],
      contract_ref: null,
      required: true,
    },
  });
}

export function observation(caseId: CaseId, runtime: RuntimeId, value?: unknown) {
  return observationEvent(caseId, runtime, value === undefined ? null : { kind: "json", value });
}

export function observe(value?: unknown, caseId = currentCaseId, effects: Effect[] = []) {
  observeCanonical(value === undefined ? null : { kind: "json", value }, caseId, "passed", effects);
}

export function observeCanonical(
  value: CanonicalValue | null,
  caseId = currentCaseId,
  status = "passed",
  effects: Effect[] = [],
) {
  if (!caseId) {
    throw new Error("Rewrit case id is missing. Call test.rewrit(...) or caseDiscovered(...) first.");
  }

  emitObservationEvent(observationEvent(caseId, runtimeId(), value, status, effects));
}

export function addEffect(effect: Effect, caseId = currentCaseId) {
  if (!caseId) {
    throw new Error("Rewrit case id is missing. Call test.rewrit(...) or caseDiscovered(...) first.");
  }

  if (lastObservation?.case_id === caseId) {
    const effects = Array.isArray(lastObservation.effects) ? lastObservation.effects : [];
    lastObservation = { ...lastObservation, effects: [...effects, effect] };
    emitEvent(lastObservation);
    return;
  }

  observeCanonical(null, caseId, "passed", [effect]);
}

export function emitObservation(caseId: CaseId, runtime: RuntimeId, value?: unknown) {
  emitObservationEvent(observation(caseId, runtime, value));
}

export function emitEvent(event: Record<string, unknown>) {
  const encoded = `${JSON.stringify(event)}\n`;
  const eventsPath = process.env.REWRIT_EVENTS_PATH;
  if (eventsPath) {
    appendFileSync(eventsPath, encoded, "utf8");
    return;
  }
  process.stdout.write(encoded);
}

export function canonicalValue(value: unknown): CanonicalValue {
  if (value === null || value === undefined) return { kind: "null" };
  if (typeof value === "boolean") return { kind: "bool", value };
  if (typeof value === "number") {
    return Number.isInteger(value)
      ? { kind: "integer", value: String(value) }
      : { kind: "float", value: String(value) };
  }
  if (typeof value === "string") return { kind: "string", value };
  if (Array.isArray(value)) return { kind: "array", items: value.map(canonicalValue) };
  if (typeof value === "object") {
    return {
      kind: "object",
      fields: Object.fromEntries(
        Object.entries(value as Record<string, unknown>).map(([key, entry]) => [key, canonicalValue(entry)]),
      ),
    };
  }
  return { kind: "string", value: String(value) };
}

function observationEvent(
  caseId: CaseId,
  runtime: RuntimeId,
  value: CanonicalValue | null,
  status = "passed",
  effects: Effect[] = [],
) {
  return {
    schema_version: "rewrit.event.v1",
    kind: "observation",
    case_id: caseId,
    runtime_id: runtime,
    status,
    value,
    error: null,
    stdout: { text: "", truncated: false },
    stderr: { text: "", truncated: false },
    exit_code: 0,
    duration_ms: 0,
    effects,
    artifacts: [],
    metadata: currentSuiteId ? { suite_id: currentSuiteId } : {},
  };
}

function emitObservationEvent(event: Record<string, unknown>) {
  lastObservation = event;
  emitEvent(event);
}

function suiteFromCaseId(caseId: string) {
  return caseId.includes(".") ? caseId.slice(0, caseId.indexOf(".")) : "default";
}
