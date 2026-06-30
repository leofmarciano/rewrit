import { caseDiscovered, emitEvent, observe, observeCanonical, runtimeId } from "./index";

type JestTestApi = (name: string, fn: () => unknown | Promise<unknown>) => unknown;

export function rewrit(caseId: string, name: string, fn: () => unknown | Promise<unknown>, testApi?: JestTestApi) {
  const runner = testApi ?? (globalThis as unknown as { test?: JestTestApi }).test;
  if (!runner) {
    throw new Error("Jest global test() is unavailable. Pass a test API to rewrit(caseId, name, fn, testApi).");
  }

  return runner(name, async () => {
    caseDiscovered(caseId, caseId.includes(".") ? caseId.slice(0, caseId.indexOf(".")) : "default", name);
    return fn();
  });
}

export { observe, observeCanonical };

export default class RewritJestReporter {
  onRunStart() {
    emitEvent({
      schema_version: "rewrit.event.v1",
      kind: "doctor_report",
      runtime_id: runtimeId(),
      report: {
        ok: true,
        checks: {
          jest_reporter: "loaded",
        },
      },
    });
  }
}
