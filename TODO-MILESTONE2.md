# Milestone 2 — Remaining Work

Implementation items are complete. The remaining validation caveat is the container executor's Unix socket feature in this network-restricted environment.

## End-to-end audit follow-up

- Persist step stdout/stderr on step records with backward-compatible serde defaults, and redact configured secret values before persistence or trace logging.
- Reset the expression environment before resolving each step so step-only variables do not bleed into later steps.
- Bound workflow timeouts to seven days and avoid integer overflow when converting minutes to seconds.
- Drain active job tasks after cancellation so executor cleanup and job status persistence can finish.
- Deliver container cancellation through the executor's cancellation signal as well as terminating the container.
- Make workspace path components collision-resistant by escaping non-alphanumeric bytes.
- Include organization-scoped in-process runs and jobs in the existing dashboard list routes, and expose their persisted output through the logs route.

## Completed

- Fixed the `JobId` ordering comparison in `forge-storage`.
- Added the direct `prost` dependency required by generated protocol code and renamed the bidirectional RPC to `Stream` to avoid tonic's generated `connect` method collision.
- Corrected Bollard's generic option types, wait-stream handling, Docker 404 matching, and executor cancellation lock scope.
- Added per-organization in-process `RunEngine`, `MemoryStore`, and `SecretsManager` instances.
- Added authenticated run trigger, status/jobs, cancellation, and secret set/list endpoints. Run status responses redact resolved job environments.
- Added engine integration coverage for success, failed-job skip propagation, timeout, cancellation, and Docker-conditional execution.
- Fixed expression parsing for `${{ ... }}` and added the missing `now_ms` / `ExecutorKind` core re-exports.
- Enforced the configured max concurrent job count with an engine semaphore.

## Validation

- `cargo test -p forge-secrets -p forge-workflow --offline` — passed (10 tests).
- `cargo test -p forge-storage -p forge-protocol --offline` — passed (2 storage tests; protocol compiled).
- `cargo check --workspace --offline` — passed with a temporary validation stub for Docker connection.
- `cargo test --workspace --offline` — passed with that same stub; the Docker-conditional integration case skipped because the stub reports Docker unavailable.
- `cargo clippy --workspace --offline` — completed with warnings.

- The final end-to-end audit edits were formatted. A fresh `cargo check -p forge-engine --offline` could not resolve the missing `hyper-named-pipe` package; the current environment also has no registry DNS access, so these latest edits could not be compiled here.

The above workspace checks used a temporary local validation setup that omitted Bollard's `pipe` feature and stubbed `ContainerExecutor::connect`; the actual Unix-socket connector and `pipe` feature have been restored. A final check with the real connector cannot resolve `hyper-named-pipe` because crates.io DNS is unavailable here. Run the workspace check/test/clippy commands with registry access to validate the actual Docker connector and regenerate `Cargo.lock` for Bollard's pipe feature.
