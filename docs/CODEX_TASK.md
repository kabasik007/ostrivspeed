# TASK FOR LOCAL CODEX — OstrivSpeed / main

You are working in `kabasik007/ostrivspeed`, branch **main only**. Do not create another branch. Read `AGENTS.md`, `README.md` and `docs/REVERSE_ENGINEERING_PLAN.md` before touching code.

## User problem
The user plays Ostriv on Windows. Generic "speedhack" tools speed up Windows timer calls; this undesirably speeds up or distorts the **camera, mouse movement, music, and sound**. Ostriv already has **built-in simulation speed controls**. Find the game's *native simulation-speed* mechanism via actual local reverse engineering and build a Rust controller for **world progress only**. Desired modes: 1x, 5x, stock maximum, 15x, 20x, 30x, 50x, 60x (subject to true simulation limits).

The objective is that buildings, workers, citizens, production, deliveries, economy and the game calendar advance faster, while audio/FM0D (FMOD), camera, UI and rendering stay real-time. Do not fake this with a 60x UI slider or Windows timer hooks.

## Execute now, in order

1. Inspect repo and local tools: `git status -sb`, `rustc --version`, `cargo --version`, available Binary Ninja, x64dbg/WinDbg, Python, Windows SDK, compiler toolchain. Detect the game executable from common installed locations; **never guess the executable path**. If unavailable, proceed with code scaffolding and report the blocker.
2. Locate `Ostriv.exe` or actual game executable; collect **version, architecture, SHA-256, module and PE information**, but never commit executable/dump/private save files. Treat untrusted trainer EXEs as untrusted; do not run them.
3. Observe stock speed controls; a historical source lists 1x/5x/12x. Confirm *installed build*. Instrument baseline: game-time/calendar change, construction/production progression per wall time, FPS, camera behavior, music speed, CPU. Use reproducible backed-up save.
4. Investigate compiled **native C++** binary with Binary Ninja; find speed UI actions -> state setter -> readers -> simulation tick/accumulator. Use debugger and memory read/write breakpoints as evidence. Confirm that the selected target is **world-simulation-specific** and not shared render/audio/input delta. Record exact module RVA, code/data type, call graph, supporting evidence and SHA-256 in `research/RE_FINDINGS.md`. If unresolved, label unverified and do NOT patch.
5. In Rust, implement a modular Windows app with read-only process discovery and version fingerprint first, then a strictly version-validated, reversible game-only controller after verification. Include egui GUI, logs, speed presets, reset 1x, auto-attach setting and safe read-only fallback. No Windows clock interception, global DLL hooking, or bypassing game licensing.
6. Measure *effective* rather than requested speed. Validate pause/resume, camera and music, save/load, production and construction, CPU limits, graceful detach/reset and unknown-build behavior. If 50/60x is impossible with this engine/hardware, document empirically why and cap appropriately.

## Architectural guidance
- Prefer `eframe/egui` + Rust stable; core separated from UI; minimal unsafe code in Windows process backend.
- Use project-specific version profiles and byte-pattern verification; do not hardcode random memory addresses.
- Add `.gitignore` excluding `target/`, local binaries, dumps, samples, research/local, save files and trainer executables.
- Put offline-only scripts for repeatable discovery in `tools/` and write the commands in `docs/`.
- Build validation: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`. Fix actual errors, don't merely propose changes.

## Deliverables
1. Working, compile-checked Rust app for Windows which at least detects and fingerprints the running game.
2. `research/RE_FINDINGS.md`: facts, hypotheses, addresses/RVAs only if verified, call graph, unknowns and steps to reproduce.
3. Game-version compatibility checks and safe state restoration; a simulation patch ONLY if proven.
4. `docs/TEST_RESULTS.md`: baseline and experimental speed measurements, including construction/camera/music observations and CPU.
5. Human-readable README: run/build/install/debug, limitations and exact next steps.

**Priority:** Verify game-specific simulation speed first; don't waste time cloning WeMod, rebuilding its EXE or making a generic Windows time speedhack. Work continuously on `main`; commit meaningful verified steps there. Report real status, not imagined completion.
