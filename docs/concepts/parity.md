# Parity

Rewrit measures behavioral parity inside observed, declared, and versioned
contracts.

It does not prove that two whole systems are identical. It answers a narrower
engineering question:

```txt
For the cases we observe and the contracts we declare, does the candidate behave
like the reference?
```

That scope is intentional. During a rewrite, the useful signal is usually not
"are these systems metaphysically equivalent?" It is "did the new implementation
change a response, error, side effect, or contract we care about?"

## Reference and Candidate

Rewrit uses neutral names:

- `reference`: the implementation treated as the source of truth.
- `candidate`: the implementation being validated.

The reference is often the legacy system, but it does not have to be. You can
compare two modern implementations, two service versions, or two refactor
branches.

Examples:

```txt
reference = Laravel/PHP
candidate = Encore/TypeScript

reference = Django/Python
candidate = Rust

reference = current HTTP API
candidate = rewritten HTTP API
```

## Cases

A case is one stable unit of behavior. The case ID is the join key between
systems, contracts, observations, reports, and waivers.

Good case IDs are domain-shaped and stable:

```txt
billing.invoice.create.success
auth.login.invalid_password
orders.refund.partial
users.profile.update_email_conflict
```

Avoid case IDs that depend on implementation details such as file names, test
method names, or framework-specific routing internals.

## How Rewrit Decides Parity

At runtime, Rewrit:

1. loads `rewrit.toml`,
2. discovers cases from the reference and candidate,
3. runs both runtimes or compares against a captured baseline,
4. collects canonical observations,
5. applies configured normalizers,
6. compares observations using the active policy,
7. applies waivers,
8. writes reports,
9. exits with a CI-friendly code.

Parity is reached when no blocking divergence remains.

Allowed waivers do not make a difference disappear. They change severity to
`allowed` and stay visible in reports until they expire.

## What Counts as Behavior?

Behavior can include:

- returned values,
- HTTP status, headers, and body,
- expected errors,
- stdout and stderr when configured,
- exit code,
- database deltas,
- files created, updated, or deleted,
- outbound HTTP calls,
- queue messages,
- domain events,
- emails,
- cache operations,
- logs,
- artifacts and metadata.

The right boundary depends on the rewrite. An HTTP API rewrite should usually
start with HTTP contracts. A function extraction may start with function-shaped
observations. A service migration may need side effects before response parity
is meaningful.

## What Rewrit Does Not Do

Rewrit does not:

- parse application source code to infer behavior,
- understand every framework inside the Rust core,
- replace unit, integration, or end-to-end tests,
- prove equivalence outside declared or observed cases,
- hide tolerated differences,
- run AI inside the engine.

Adapters and SDKs know about test runners and frameworks. The core compares
canonical observations.
