# Reverse engineering plan: native Ostriv simulation speed

## Explicit outcome
Increase *world simulation* time and work progression (buildings, agriculture, manufacturing, trade, citizens, game calendar) while leaving **rendering, FPS pacing, camera controls, mouse/keyboard input, UI and FMOD music/SFX** unchanged. Target configurable 1x, 5x, 12x, 20x, 30x, 50x, 60x **if real simulation throughput permits**. Never promise actual 60x before measurement.

## Known starting evidence (independent, not yet tested locally)
- Official FAQ says Ostriv was written in C++17 and uses its own engine, OpenGL and FMOD: https://ostrivgame.com/faq/
- A 2021 control reference lists 1x (key 1), 5x (key 2), 12x (key 3): https://www.magicgameworld.com/ostriv-pc-keyboard-controls-shortcuts/
- Steam players report that at 12x, actual construction throughput can regress due to hardware/simulation limits: https://steamcommunity.com/app/773790/discussions/0/600770233128486436/
- These are clues; **live installed-game evidence overrides all of them**.

## 0. Environment and reproducibility
- Discover Ostriv exe and installed version. Ask for the path only if discovery fails.
- Detect local Rust, MSVC build tools, Binary Ninja and optional x64dbg/WinDbg/Cheat Engine. Do not download/run unknown trainers.
- Capture SHA-256 and PE bitness. Place all private, local reports in ignored `research/local/`.
- Back up saves and, if patching in memory, have a clean restart strategy.
- Initial acceptance: Rust program can identify Ostriv process and show PID/executable/hash, without altering anything.

## 1. Behavioral baseline
At each actual stock speed, save the same scenario and measure over a fixed real-time interval:
- in-game calendar delta;
- count/completion progress of a selected construction task;
- number of citizen/work operations;
- camera movement per real-time second;
- music playback tempo and sound latency;
- frame rate, CPU utilisation and simulation hitching.
Record machine specs and conditions (pause, UI focus, population).

Define `effective_game_speed = game_time_elapsed / wall_clock_elapsed` with consistent units. Also report `effective_construction_speed` separately. They may diverge.

## 2. Locate the native speed mechanism — evidence first
A. Start live game and identify actual controls/state: pause / 1x / 5x / 12x (or changed stock speeds).

B. Inspect binary in Binary Ninja:
- inspect imports / event and input dispatch;
- trace keybind actions and pause/unpause state;
- examine string cross-references and numeric constants around speed selection;
- seek simulation scheduler / fixed-step accumulator / world update loops;
- find references to candidate speed state and distinguish readers for UI and world;
- do not infer a function solely from constants without runtime confirmation.

C. Confirm dynamically (debugger or read-only memory search first):
- compare candidate state changes between pause and presets;
- set read/write breakpoints on candidate variables;
- capture call stack and disassembly at the actual setter + tick consumer;
- determine whether values represent enum, float multiplier, tick budget, frame delta or other state;
- confirm calendar, production and construction are driven by this path.

D. Determine safe intervention:
- Preferred: change an *internal simulation-only multiplier/tick budget* using build-verified read/write, if independent of real-time systems.
- Fallback: narrow, game-specific hook at verified simulation code path, with guarded detach and independent camera/audio timing.
- If same delta feeds camera or audio: **do not patch it**; trace deeper to world-simulation-only processing.
- Consider tick clamping, `dt` upper limits, job queues, thread sync, frame budget, timers and simulation backlog. Avoid forcing huge `dt` values across unrelated systems.

For each found function write a `research/RE_FINDINGS.md` entry with binary hash, module+RVA, evidence, type, xrefs, screenshots/log references (stored locally), verification status and potential failure mode. Use module-relative addresses or verified signatures, never hardcode absolute ASLR addresses without version checks.

## 3. Rust implementation
- Start with a *read-only* Windows process inspector (`EnumProcesses`/`CreateToolhelp32Snapshot`, etc.) behind `ProcessBackend` abstraction.
- Verify hash + expected byte sequences before any mutation. If not recognized, disable speed editing.
- Implement simulation speed controller with clamp/config, apply, readback, restore and state tracking.
- Implement Rust egui/eframe GUI: process indicator, version, current internal speed, speed presets, optional slider up to 60x, 1x reset, attach/detach, read-only/research mode, logs and warnings.
- Keep Win32 unsafe code isolated. No generic speedhack library or system-wide timer override.
- No unexplained injection, persistence, obfuscation or unnecessary admin privileges.

## 4. Validation ladder
1x -> 5x -> stock max -> 15x -> 20x -> 30x -> 50x -> 60x, one increment at a time.

Must test at each new threshold:
- Audio music pitch and playback speed unchanged.
- Camera WASD/panning/zoom speed per wall-clock second unchanged.
- UI responsiveness and mouse remain normal.
- Building/citizens/economy/calendar speed genuinely increases over baseline.
- Pause/resume, loading saves, save integrity, detach, game exit and reattach work.
- CPU load and task backlog remain bounded. If throughput plateaus or instability emerges, show 'requested X / effective Y' and recommend lower speed.

A real 60x is a stretch goal. Never substitute accelerated wall-clock timer APIs for unmet simulation throughput.

## Milestones and reporting
- **M0**: Rust Windows scaffold, executable fingerprint, detection, read-only status, build/tests, logs.
- **M1**: Proven reverse-engineering call graph and candidate simulation-only control; annotate assumptions.
- **M2**: Build-verified reversible 1x/stock-speed patch with preservation of camera/audio.
- **M3**: 15x-60x experimental settings with measured throughput, warnings and guardrails.
- **M4**: GUI polish, release packaging, docs and regression checks.

If no game binary/debugger is locally accessible, complete M0 and detailed RE scripts/checklists, then report precisely which local inputs are missing. Never invent M1-M3 success.

## Legal, scope and security
Single-player owned copy only. Do not redistribute proprietary executable, decompiled code, personal user data, trainer binary or original assets. Do not bypass license checks, DRM or anti-cheat. Do not use unknown trainer EXEs as necessary runtime dependencies.
