# Local test results

Build: Ostriv `0.5.9.65`, Steam build `26211342`, SHA-256 `d07fdcde38cc4a1e57696332b68ff6f74bf8fb931659b71b414d28c1a89d8be0`.

| Check | Result |
|---|---|
| Stock buttons | Three visible; RAM target/current doubles changed 1.0 → 5.0 → 12.0 → 1.0 under user control. |
| 1× timed baseline | Accumulated world time advanced `10.000` units in `10.000` real seconds. |
| 15× experimental, twice | Target readback `15.0`; restore readback `1.0` after each 15-second interval. User reported faster world simulation, normal music and camera. |
| 15× timed while apparently paused | Target readback `15.0`; world time delta `0.000` over 10 seconds, current speed `1.0`. Not a throughput result. |
| Active 1×, 8 s | `8.007` world units / `8.000` real s = `1.00×`; process CPU `4.86` core-seconds. |
| Active 15×, 8 s | `114.972` world units / `8.000` real s = `14.37×`; CPU `7.81` core-seconds; current 15× at end; target restored 1×. |
| Active 20×, 8 s | `151.003` world units / `8.000` real s = `18.87×`; CPU `6.92` core-seconds; current 20×; restored 1×. |
| Active 30×, 8 s | `218.538` world units / `8.001` real s = `27.32×`; CPU `7.06` core-seconds; current 30×; restored 1×. |
| Active 50×, 8 s | `339.764` world units / `8.000` real s = `42.47×`; CPU `7.69` core-seconds; current 50×; restored 1×. |
| 60×, 8 s | Only `1.003` world units / `8.000` real s = `0.13×`; current speed 0× at end, target restored 1×. A read-only snapshot afterwards still showed current 0×, target 1×. The user reports the game stopped or hung without manual pause, then recovered. 60× is disabled in the controller. |
| 50× audio/camera | User reports music and camera remained normal; game recovered after the later 60× stop. |
| Construction, production, economy, save/load, transport | Not yet measured. |
| Binary Ninja debugger watchpoints | Not yet captured; Binary Ninja Free is installed but no headless API is available. |
| `cargo fmt --all -- --check` | Passed after formatting. |
| `cargo clippy --offline --all-targets -- -D warnings` | Passed. |
| `cargo test --offline` | 2 unit tests passed (PE architecture parser, unknown-build refusal). |
| Windows release build | Final `cargo build --offline --release` passed. Release `--status` recognized the running game; release `--restore` returned readback 1×. |

The trial mode writes only the confirmed target-speed double in the live game process, reads it back, and restores 1× when the timed test ends. The executable file on disk was not changed. The Codex sandbox denied `PROCESS_VM_WRITE`; the explicitly escalated local run succeeded. A normal user-launched GUI requires Windows process-write access to the game. The controller also checks the executable's setter bytes and initial target bytes in addition to its full-file hash.
