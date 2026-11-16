# GitHub Actions CI evidence — commit 40167f3d18a177b68df31e82ea2243d0351af817

Run: https://github.com/northwaterzuma/rustcatch/actions/runs/43845781548
Triggered by push of commit 40167f3 to branch rustcatch/dev-v1.
Result: **completed / success** (2026-11-16T10:11Z approx).

## Jobs (all success)

| Job | Conclusion | Job ID |
|---|---|---|
| Contract Schema & Offline Protocol Suite | success | 133225471393 |
| Rust Build & Test (windows-latest, x86_64-pc-windows-msvc) | success | 133225576667 |
| Rust Build & Test (ubuntu-latest, x86_64-unknown-linux-gnu) | success | 133225576557 |
| Rust Build & Test (macos-latest, aarch64-apple-darwin) | success | 133225576586 |

Each Rust job ran `cargo check --workspace --target <t>`, `cargo test --workspace --target <t>`,
and `cargo build --release --workspace --target <t>` per .github/workflows/ci.yml.

## Real Rust test totals (parsed from Windows job log)

35 passed, 0 failed across the workspace:
- crates/ipc unit tests: 4 passed
- crates/app/tests/scheduler_logic.rs: 3 passed
- crates/engine/tests/product_logic.rs: 20 passed
- crates/tools/tests/media_logic.rs: 8 passed

Coverage includes actual product functions: HTTP Range/206/200/416 validation,
ETag/validator guards, segment-planner generation fencing, durable checkpoint
roundtrip + higher-schema refusal, crash-window reconciliation, yt-dlp 12-way
error classification, MediaIntent format selection, and scheduler occurrence dedup.

## Prior failing runs (root causes, all fixed)

- 43842660189: compile failure in 5 files (literal CR/LF, unescaped quotes, Rust-2024 `gen` keyword)
- 43844382676: `test_decimal_u64_bounds` panic (bare integer vs JSON string) on all 3 platforms
- 43844950258: `test_weak_etag_is_not_strong_validator` assertion mismatch + 3 latent media test assertion bugs

## Boundaries

These results verify Rust compilation, unit/integration tests and release builds on
three GitHub-hosted toolchains. They do NOT verify GUI/IDM visual parity, real browser
handoff, authorized YouTube downloads, installer/signing, FTPS/AD controlled-domain
scenarios, or macOS/Linux desktop behavior — those remain needs_env per ACCEPTANCE.md.
