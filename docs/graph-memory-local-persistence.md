# Graph Memory Local Persistence Notes

This note records the current local-backend findings for the governed Graph Memory functional-alpha path.

The target remains narrow: explicit local persistence for approved operational/project memory and FailureInsight readback, without broad autonomous memory writes or readback-as-authorization.

## Current working baseline

`crates/graph-memory` currently uses SurrealDB with the in-memory local backend (`kv-mem`) for tests and the local FailureInsight demo path.

That baseline is useful for proving:

```text
signal -> proposal -> Decision Gate -> audit -> approved persistence in the demo store -> readback proof
```

It does not yet prove persistence across separate process invocations.

## Backend evaluation from the focus loop

Two obvious SurrealDB local persistence backends were checked before implementation:

1. `kv-surrealkv`
   - exposes `surrealdb::engine::local::SurrealKV` in SurrealDB 1.5.x;
   - requires compiling SurrealDB with the `surrealdb_unstable` cfg flag;
   - plain `cargo check` fails with:

   ```text
   `kv-surrealkv` is currently unstable. You need to enable the `surrealdb_unstable` flag to use it.
   ```

2. `kv-rocksdb` / `File`
   - exposes `surrealdb::engine::local::{File, RocksDb}`;
   - pulls native RocksDB/zstd build dependencies;
   - local verification failed in the scheduled environment while building `zstd-sys` because clang could not find `stddef.h`.

Because the live focus loop requires plain:

```bash
cargo fmt -- --check
cargo check
cargo test
```

neither backend should be made part of the always-on workspace baseline until the build story is explicit and verified.

## Safe implementation: demo snapshot path (2026-05-24)

The current solution adds a **demo snapshot path** — a pure-Rust JSON file persistence mechanism that does not depend on native SurrealDB backends.

Added:

- `crates/graph-memory/src/demo_snapshot.rs`: `FailureInsightDemoSnapshot` struct with JSON serialize/deserialize, `write_to_file()` and `read_failure_insight_demo_snapshot()` functions.
- `crates/cli/src/main.rs`: `--snapshot-path <path>` flag on `memory demo failure-insight`, and a new `memory demo snapshot-read <path>` subcommand for cross-invocation readback.

Usage:

```bash
# Run the demo and save a snapshot
cargo run -q --bin arpagona -- memory demo failure-insight --json --snapshot-path target/demo-snapshot.json

# In a separate process, read the snapshot back
cargo run -q --bin arpagona -- memory demo snapshot-read target/demo-snapshot.json
```

This proves cross-invocation readback without unstable cfg flags or native RocksDB/zstd dependencies. It is a development-only proof, not a production persistence mechanism.

## Future safe implementation options

Any implementation still must preserve:

```text
ProposedAction -> DecisionGate -> Decision -> Audit -> controlled effect only if approved
```

and must keep local readback proof non-authorizing.

Future options (in priority order):

1. **Opt-in Cargo feature** for local persistent Graph Memory that is disabled by default and has its own documented verification command.
2. **Crate-local persistence abstraction** whose default implementation remains `kv-mem` and whose local file-backed implementation is gated and tested separately, once the RocksDB/zstd-sys build story is resolved (e.g., via `LIBCLANG_PATH` or system package).
