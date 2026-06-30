# Security

Rewrit executes project code through adapters. Treat configured runtimes as
trusted project code by default.

Baseline protections:

- explicit working directory per runtime,
- runtime timeout,
- stdout and stderr byte limits,
- secret redaction in captured output,
- temporary `.rewrit/tmp` area,
- optional Docker or Podman sandboxing through `[security.sandbox]`.

Sandboxing is disabled by default and is intended for trusted images configured
by the project. Rewrit does not fetch or build images automatically.

Report vulnerabilities privately through the project security contact before
public disclosure.
