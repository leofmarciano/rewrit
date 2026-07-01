# @rewrit/node

Node SDK and test-runner helpers for the Rewrit parity engine.

```bash
npm install @rewrit/node
```

```ts
import { caseDiscovered, observe } from "@rewrit/node";

caseDiscovered("billing.invoice.create.success", "billing");
observe({ status: "open", amount: "199.90" });
```

The SDK emits Rewrit adapter protocol events to `REWRIT_EVENTS_PATH` when it is
set, or to stdout otherwise.

Exports:

- `@rewrit/node`
- `@rewrit/node/vitest-reporter`
- `@rewrit/node/jest-reporter`
- `@rewrit/node/encore`

## Publishing

```bash
npm ci
npm run build
npm publish --access public
```

For GitHub Packages, packages must be scoped to the GitHub owner namespace. The
manual `publish-node` workflow publishes this SDK as
`@<github-owner>/rewrit-node`; in this repository that is
`@leofmarciano/rewrit-node`.

To publish manually to GitHub Packages, authenticate against
`https://npm.pkg.github.com`, set the package name to your owner scope, and run:

```bash
npm publish --registry=https://npm.pkg.github.com
```

The workflow can publish to npm, GitHub Packages, or both.
