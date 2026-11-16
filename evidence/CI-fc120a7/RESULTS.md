# GitHub Actions CI evidence — commit fc120a782adc24257f5a83f9f298c0bdbe05b844

Run: https://github.com/northwaterzuma/rustcatch/actions/runs/43848343635
Result: completed / success — all 4 jobs green.

## Jobs
| Job | Conclusion | Job ID |
|---|---|---|
| Contract Schema & Offline Protocol Suite | success | 133234169041 |
| Rust Build & Test (windows-latest, msvc) | success | 133234299877 |
| Rust Build & Test (macos-latest, aarch64) | success | 133234299962 |
| Rust Build & Test (ubuntu-latest) | success | 133234300108 |

## Real Rust test totals (per-platform, identical on all 3 targets)
54 passed, 0 failed:
- crates/ipc unit tests: 4
- crates/app/tests/scheduler_logic.rs: 3
- crates/engine/tests/product_logic.rs: 20
- crates/tools/tests/media_logic.rs: 8
- crates/tools/tests/streams_and_proc.rs: 9 (NEW in this commit — TEST-05B/06/07A/12A)
- crates/nmhost/tests/handoff_protocol.rs: 10

All three targets also completed `cargo check`, `cargo test` and
`cargo build --release --workspace`, and uploaded platform artifacts.

## Boundaries
CI verifies Rust build/test/release on 3 toolchains. It does NOT verify real Edge/
Chrome native messaging, GUI visual parity, authorized YouTube downloads, installer
signing, controlled FTPS/AD domains, or macOS/Linux desktop behavior. Those stay
needs_env.
