import { caseDiscovered, emitEvent, observe, observeCanonical, runtimeId } from "./index";
import { test as baseTest } from "vitest";

type TestFn = (...args: unknown[]) => unknown | Promise<unknown>;
type VitestTest = typeof baseTest & {
  rewrit: (caseId: string, name: string, fn: TestFn) => unknown;
};

export const test = Object.assign(baseTest, {
  rewrit(caseId: string, name: string, fn: TestFn) {
    return baseTest(name, async (...args: unknown[]) => {
      caseDiscovered(caseId, caseId.includes(".") ? caseId.slice(0, caseId.indexOf(".")) : "default", name);
      return fn(...args);
    });
  },
}) as VitestTest;

export { observe, observeCanonical };

export default class RewritVitestReporter {
  onInit() {
    emitEvent({
      schema_version: "rewrit.event.v1",
      kind: "doctor_report",
      runtime_id: runtimeId(),
      report: {
        ok: true,
        checks: {
          vitest_reporter: "loaded",
        },
      },
    });
  }
}
