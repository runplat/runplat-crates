# Runplat Crates

Collection of crates for runplat projects. Runplat is a framework is an extensible plugin-based framework for building and maintaining runtimes.

## Crates

- `runir`: Library for building a runtime intermediate representation for an application. Exports a thread-safe runtime object store that supports opt-in content addressable storage at runtime.
- `reality`: Framework for building a plugin system built on top of the `runir` store, using "call-by-name" semantics for executing plugin logic.
- `kioto`: Framework providing building blocks for creating event-driven engine's built on top of `reality` plugins and `tokio` runtime.
- `runplat-macros`: Helper macros supporting the above crates
