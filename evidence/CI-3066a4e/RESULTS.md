# GitHub Actions CI evidence — commit 3066a4e58eecb2d39ff49a6a181c462c3e8476ff

Run: https://github.com/northwaterzuma/rustcatch/actions/runs/43847154705
Result: completed / success — all 4 jobs green.

## Jobs
| Job | Conclusion | Job ID |
|---|---|---|
| Contract Schema & Offline Protocol Suite | success | 133230087417 |
| Rust Build & Test (windows-latest, msvc) | success | 133230198281 |
| Rust Build & Test (ubuntu-latest) | success | 133230198130 |
| Rust Build & Test (macos-latest) | success | 133230198238 |

## Real Rust test totals (Windows job log)
45 passed, 0 failed:
- crates/ipc unit tests: 4
- crates/app/tests/scheduler_logic.rs: 3
- crates/engine/tests/product_logic.rs: 20
- crates/tools/tests/media_logic.rs: 8
- crates/nmhost/tests/handoff_protocol.rs: 10 (NEW)

The 10 new tests call NativeHostSession::handle_raw_frame end-to-end:
untrusted origin rejection, pre-auth op rejection, unsupported version,
4-stage HMAC handshake + single-use challenge, prepare durable/HandoffPending
zero-transfer, commit-requires-cancel-confirmation, same-key idempotency,
POST/unknown-replayability rejection, orphan commit rejection, redacted
task.get snapshot, and session profile-switch rejection.

## Boundaries
CI verifies Rust build/test/release on 3 toolchains. It does NOT verify real Edge/
Chrome native messaging, GUI visual parity, authorized YouTube downloads, installer
signing, controlled FTPS/AD domains, or macOS/Linux desktop behavior.
