# Security

Rewrit executes project code through adapters. Treat configured runtimes as
trusted project code by default.

Baseline protections:

- explicit working directory per runtime,
- runtime timeout,
- stdout and stderr byte limits,
- secret redaction in captured output,
- temporary `.rewrit/tmp` area,
- no Docker or Podman sandbox requirement in MVP 1.

Report vulnerabilities privately through the project security contact before
public disclosure.

