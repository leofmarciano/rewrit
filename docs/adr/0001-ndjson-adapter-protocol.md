# ADR 0001: NDJSON Adapter Protocol

Adapters emit versioned NDJSON events. This gives streaming, easy debugging,
simple cross-language support and avoids FFI or framework-specific coupling.

