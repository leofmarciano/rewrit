export type RuntimeId = string;
export type CaseId = string;

export function observation(caseId: CaseId, runtimeId: RuntimeId, value?: unknown) {
  return {
    schema_version: "rewrit.event.v1",
    kind: "observation",
    case_id: caseId,
    runtime_id: runtimeId,
    status: "passed",
    value: value === undefined ? undefined : { kind: "json", value },
    error: null,
    stdout: { text: "", truncated: false },
    stderr: { text: "", truncated: false },
    exit_code: 0,
    duration_ms: 0,
    effects: [],
    artifacts: [],
    metadata: {},
  };
}

export function emitObservation(caseId: CaseId, runtimeId: RuntimeId, value?: unknown) {
  process.stdout.write(`${JSON.stringify(observation(caseId, runtimeId, value))}\n`);
}

