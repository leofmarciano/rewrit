import { caseDiscovered, emitEvent, observe, observeCanonical, runtimeId } from "./index.ts";

type TestFn = (...args: unknown[]) => unknown | Promise<unknown>;
type VitestBaseTest = (name: string, fn: TestFn) => unknown;
type VitestTest = VitestBaseTest & {
  rewrit: (caseId: string, name: string, fn: TestFn) => unknown;
};

export function createRewritTest(baseTest: VitestBaseTest): VitestTest {
  return Object.assign(baseTest, {
    rewrit(caseId: string, name: string, fn: TestFn) {
      return baseTest(name, async (...args: unknown[]) => {
        caseDiscovered(caseId, caseId.includes(".") ? caseId.slice(0, caseId.indexOf(".")) : "default", name);
        return fn(...args);
      });
    },
  }) as VitestTest;
}

export const test = createRewritTest((name: string, fn: TestFn) => {
  const baseTest = (globalThis as unknown as { test?: VitestBaseTest }).test;
  if (!baseTest) {
    throw new Error("Vitest global test() is unavailable. Use createRewritTest(test) with the Vitest test API.");
  }

  return baseTest(name, fn);
});

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
