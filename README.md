# OstrivSpeed

**Native simulation-speed research and Rust controller for Ostriv (Windows, single-player).**

## Current result

For the installed Steam build `26211342` (`Alpha 5 patch 9`, `0.5.9.65`), controlled RAM snapshots and static disassembly identified the game's own target speed, current speed and world-time update path. Two reversible 15× trials in a running game succeeded; the user observed faster world progression with normal camera and music. See [the evidence ledger](research/RE_FINDINGS.md) and [local test results](docs/TEST_RESULTS.md). Higher speeds remain experimental and must be judged by **effective world-time progression**, not by the requested value alone.

The file named `Ostriv Trainer.exe` in this local checkout was identified as a WeMod installer, not an Ostriv trainer. It is ignored by Git and was not executed.

## Build and run

Requires Windows x64 and stable Rust. From the repository root:

```powershell
cargo build --release
target\release\ostrivspeed.exe --status
target\release\ostrivspeed.exe
```

`--status` discovers a running `ostriv.exe` and prints PID, executable path, architecture, SHA-256 and whether its build is recognized. The dark GUI shows a speed slider, preset buttons, live target/current/effective speed, process discovery, build verification and diagnostics. Enable control for the session, then attach before changing speed. Unknown builds cannot attach. If Windows denies process-write access, launch the app with matching integrity to the game; do not disable Windows security controls.

The dashboard layout takes inspiration from [OpenSpeedy](https://github.com/game1024/OpenSpeedy), but uses original Rust/egui code and Ostriv's verified game variable. It does not copy OpenSpeedy code or its Windows timer interception. Registered global shortcuts: `Ctrl+Alt+0` resets to 1×; `Ctrl+Alt+1` through `Ctrl+Alt+7` select 1×, 5×, 12×, 15×, 20×, 30×, 50×; `Ctrl+Alt+Up/Down` steps through presets. Shortcuts change speed only while attached. `--hotkey-check` reports how many shortcuts Windows accepted.

For a bounded measurement that automatically attempts to restore 1×:

```powershell
target\release\ostrivspeed.exe --experimental-speed 15 10
target\release\ostrivspeed.exe --restore
```

The first argument is one of `1, 5, 12, 15, 20, 30, 50`; the second is a duration of 1–30 seconds. A completed run prints target readback, current speed, world-time delta, effective rate and process CPU time. The game must be unpaused. Use a backed-up save for experiments, especially beyond 15×. `60×` was disabled after the live game stopped responding at that setting. Normal exit and detach restore 1× if the app's last value is still present; a game restart also loads its default speed. After an abrupt controller termination, use `--restore` or a stock speed button.

## Research workflow

`tools/snapshot_data.py` reads the verified game's `.data` section from RAM without writing it. Capture the three stock modes into ignored `research/local/` files, then run `tools/compare_data.py` to find values that follow `1.0 → 5.0 → 12.0`. The scripts accept the actual game executable path and module base; do not commit snapshots or disassembly.

```powershell
$game = Get-Process ostriv
$base = '0x{0:X}' -f $game.MainModule.BaseAddress.ToInt64()
python tools/snapshot_data.py $game.Id $base $game.Path research/local/one.bin
# Switch to 5× and capture five.bin, then 12× and capture twelve.bin.
python tools/compare_data.py research/local/one.bin research/local/five.bin research/local/twelve.bin
```

The target RVA and expected executable hash in the Rust controller apply **only** to the build documented above. A new game update needs fresh static and dynamic verification. No Windows timer API is hooked, and no game file is modified on disk.

## Objective
Change **only Ostriv's own simulation progression** (building, economy, production, citizens, in-game calendar) beyond the existing speed presets while **leaving camera navigation, rendering, UI interaction, music and FMOD audio at normal real-time speed**.

The goal is to discover the internal speed switch/multiplier and update/tick loop through evidence-based reverse engineering. No global Windows timer hooks, no DLL patching of system timer APIs, no generic speedhack. An exposed multiplier does not prove 50x or 60x is stable: measure throughput and tick backlog.

## Facts and constraints
- Ostriv uses a custom C++17 engine, OpenGL and FMOD. Official FAQ: https://ostrivgame.com/faq/
- Older keybinding references describe 1x / 5x / 12x game speeds; confirm for the **installed version**, rather than assuming a 3x ceiling.
- Existing trainer EXEs should be treated as untrusted research samples, never executed by default.
- Research only a locally owned single-player copy. Do not bypass licensing, distribute proprietary binaries or upload game files.
- **Work on branch `main` only.** No feature branches.
- Do not edit `Ostriv.exe` on disk as the default approach. Back up saves, favor runtime changes that can be reverted.

## Start here
1. Read [AGENTS.md](AGENTS.md) for the local Codex workflow.
2. Follow [docs/REVERSE_ENGINEERING_PLAN.md](docs/REVERSE_ENGINEERING_PLAN.md) to find and validate the game's native simulation-speed mechanism.
3. Use [docs/CODEX_TASK.md](docs/CODEX_TASK.md) as the ready-to-run local Codex brief.

**Status:** 15×, 20×, 30× and 50× advanced measured world time in one scenario. The user observed normal music and camera at 15× and 50×. A 60× trial stopped the game and is disabled. Construction/production throughput, save/load, and long-run stability remain to be measured.
