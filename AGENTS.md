# Repository Guidelines

## Project Structure & Module Organization
- `crates/` holds the Cargo workspace (shared core utilities, DNS codec, and the client/server CLIs).
- `docs/` contains public docs and design notes.
- `fixtures/vectors/` stores golden DNS vectors used by Rust tests.
- `.github/certs/` holds test TLS certs/keys for interop and benchmarks.
- `tools/vector_gen/` contains the C vector generator and its CSV input.
- `scripts/` includes vector generation and interop harness utilities (`scripts/interop/`).
- `.interop/` is runtime output for captures/builds and should remain untracked.

## Build, Test, and Development Commands
- `cargo test -p slipstream-dns` runs the vector-based DNS codec suite.
- `cargo test` runs all Rust tests in the workspace.
- `cargo fmt` formats Rust code; run before committing.
- `cargo run -p slipstream-client -- --resolver=IP:PORT --domain=example.com` runs the client CLI.
- `cargo run -p slipstream-server -- --target-address=IP:PORT --domain=example.com` runs the server CLI.
- `./scripts/gen_vectors.sh` regenerates `fixtures/vectors/dns-vectors.json` from the C implementation.
- `cargo build -p slipstream-dns --bin bench_dns --release` builds the DNS codec microbench; run `/usr/bin/time -v ./target/release/bench_dns --iterations=20000 --payload-len=256` for timing + RSS stats.
- `TRANSFER_BYTES=10485760 ./scripts/bench/run_rust_rust_10mb.sh` runs the Rust↔Rust 10MB exfil/download benchmark (tune `SOCKET_TIMEOUT` if transfers are slow).
- `RUN_DOWNLOAD=0 TRANSFER_BYTES=104857600 ./scripts/bench/run_rust_rust_10mb.sh` runs an exfil-only 100MB benchmark; increase `SOCKET_TIMEOUT` for long runs.
- `RUNS=20 RUN_DOWNLOAD=0 TRANSFER_BYTES=104857600 ./scripts/bench/run_rust_rust_10mb.sh` repeats the exfil benchmark for reliability checks.
- The Rust client/server link to picoquic via `slipstream-ffi`; by default use the `vendor/picoquic` submodule and build with `./scripts/build_picoquic.sh` (outputs to `.picoquic-build/`, fetches picotls unless `PICOQUIC_FETCH_PTLS=OFF`). `cargo build` will auto-run the script when libs are missing; set `PICOQUIC_AUTO_BUILD=0` to disable. You can also set `PICOQUIC_DIR` (headers) plus `PICOQUIC_BUILD_DIR` or `PICOQUIC_LIB_DIR` (libs).

## Coding Style & Naming Conventions
- Indentation: 4 spaces in C/Python, 2 spaces in shell scripts; Rust uses `cargo fmt`.
- Keep bash scripts in strict mode (`set -euo pipefail`) and use descriptive variable names.
- Use ASCII by default; keep filenames and Rust modules in `snake_case`.
- Prefer small, focused Rust modules and explicit error handling over panics.

## Testing Guidelines
- `fixtures/vectors/dns-vectors.json` is the source of truth for DNS behavior.
- `crates/slipstream-dns/tests/vectors.rs` must pass for DNS changes.
- Interop harness captures in `.interop/` are used for manual verification.
- When protocol behavior changes, update vectors and `docs/protocol.md` plus `docs/dns-codec.md`.
- Interop suites:
  - `./scripts/interop/run_local.sh` (C↔C baseline; needs `SLIPSTREAM_DIR`).
  - `./scripts/interop/run_rust_client.sh` (Rust client ↔ C server; needs `SLIPSTREAM_DIR`).
  - `./scripts/interop/run_rust_server.sh` (C client ↔ Rust server; needs `SLIPSTREAM_DIR`).
  - `./scripts/interop/run_rust_rust.sh` (Rust ↔ Rust; uses local picoquic + `.github/certs`).

## Commit & Pull Request Guidelines
- Keep commit messages short and imperative.
- PRs should include a brief summary, rationale, and commands run.
- Regenerate vectors and update docs when DNS behavior or CLI defaults change.
