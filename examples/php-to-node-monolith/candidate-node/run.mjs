import { appendFileSync } from "node:fs";

const runtimeId = process.env.REWRIT_RUNTIME_ID || "candidate_node";
const command = process.env.REWRIT_ADAPTER_COMMAND || "run";
const caseId = "catalog.product.price.success";

function emit(event) {
  const encoded = `${JSON.stringify(event)}\n`;
  if (process.env.REWRIT_EVENTS_PATH) {
    appendFileSync(process.env.REWRIT_EVENTS_PATH, encoded, "utf8");
    return;
  }
  process.stdout.write(encoded);
}

function caseDiscovered() {
  emit({
    schema_version: "rewrit.event.v1",
    kind: "case_discovered",
    runtime_id: runtimeId,
    case: {
      id: caseId,
      suite_id: "catalog",
      title: "calculates product price",
      source_location: null,
      tags: [],
      contract_ref: null,
      required: true,
    },
  });
}

if (command === "doctor") {
  emit({
    schema_version: "rewrit.event.v1",
    kind: "doctor_report",
    runtime_id: runtimeId,
    report: { ok: true, checks: { node: process.version } },
  });
  process.exit(0);
}

caseDiscovered();

if (command === "discover") {
  process.exit(0);
}

emit({
  schema_version: "rewrit.event.v1",
  kind: "observation",
  case_id: caseId,
  runtime_id: runtimeId,
  status: "passed",
  value: {
    kind: "json",
    value: {
      sku: "sku_123",
      unit_price: "49.95",
      quantity: 2,
      total: "99.90",
      currency: "BRL",
    },
  },
  error: null,
  stdout: { text: "", truncated: false },
  stderr: { text: "", truncated: false },
  exit_code: 0,
  duration_ms: 1,
  effects: [],
  artifacts: [],
  metadata: { suite_id: "catalog" },
});
