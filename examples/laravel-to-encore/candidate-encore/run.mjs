import { appendFileSync } from "node:fs";

const runtimeId = process.env.REWRIT_RUNTIME_ID || "encore_ts";
const command = process.env.REWRIT_ADAPTER_COMMAND || "run";

function emit(event) {
  const encoded = `${JSON.stringify(event)}\n`;
  if (process.env.REWRIT_EVENTS_PATH) {
    appendFileSync(process.env.REWRIT_EVENTS_PATH, encoded, "utf8");
    return;
  }
  process.stdout.write(encoded);
}

function caseDiscovered(caseId, suiteId, title) {
  emit({
    schema_version: "rewrit.event.v1",
    kind: "case_discovered",
    runtime_id: runtimeId,
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

function text(value) {
  return { kind: "string", value };
}

if (command === "doctor") {
  emit({
    schema_version: "rewrit.event.v1",
    kind: "doctor_report",
    runtime_id: runtimeId,
    report: { ok: true, checks: { node: process.version, encore: "fixture" } },
  });
  process.exit(0);
}

caseDiscovered("billing.invoice.create.success", "billing", "creates invoice");

if (command === "discover") {
  process.exit(0);
}

emit({
  schema_version: "rewrit.event.v1",
  kind: "observation",
  case_id: "billing.invoice.create.success",
  runtime_id: runtimeId,
  status: "passed",
  value: {
    kind: "object",
    fields: {
      status: { kind: "integer", value: "201" },
      headers: {
        kind: "object",
        fields: {
          "content-type": text("application/json"),
          "x-request-id": text("encore-request-id"),
        },
      },
      body: {
        kind: "json",
        value: {
          id: "inv_123",
          amount: "199.90",
          currency: "BRL",
          status: "open",
        },
      },
    },
  },
  error: null,
  stdout: { text: "", truncated: false },
  stderr: { text: "", truncated: false },
  exit_code: 0,
  duration_ms: 1,
  effects: [
    {
      kind: "db_delta",
      connection: "default",
      table: "billing_invoices",
      inserted: [
        {
          invoice_id: text("inv_123"),
          customer_ref: text("cus_123"),
          total_amount: text("199.90"),
          currency_code: text("BRL"),
          state: text("open"),
        },
      ],
      updated: [],
      deleted: [],
    },
  ],
  artifacts: [],
  metadata: { suite_id: "billing" },
});
