# Agent instructions — OstrivSpeed (read first)

## Mission
Build a clean Windows Rust tool that controls **Ostriv's own simulation speed**, not operating-system time. The accelerated subsystems are the in-game calendar, construction, citizens, economy, manufacturing and logistics. Audio (FMOD), music playback, camera motion, mouse/keyboard input, menus and rendering must remain at their native speed.

## Git discipline
- Repository: `kabasik007/ostrivspeed`; work **only on `main`**. Never create or switch to a feature branch or open an unnecessary PR.
- Make small, descriptive commits and keep a changelog of actual evidence.
- Don't add copyrighted Ostriv assets, original game binaries, trainer EXEs, dumps, license keys, or personal paths to Git.
- Never reuse or copy PHP-specific components from `kabasik007/reverseeng`. `kabasik007/Awesome-REA` is a **reference catalog**, not a native disassembler.

## Local-only reverse-engineering protocol
1. Inspect locally installed tools and available game files, without running untrusted external trainer downloads.
2. Record game executable filename, PE architecture, size, SHA-256, version/hash and module layout in an ignored local `research/local/` directory. Treat any trainer as a read-only specimen.
3. Record behavior at each **actual installed in-game speed preset**. Some references list 1x, 5x, 12x. Do not assume older values still apply. Measure real seconds versus game-time progression, construction, citizen movement, camera and audio.
4. Find the in-game speed setter and simulation-update/tick consumer using Binary Ninja, dynamic debugger (if available), and observed state changes; document evidence and uncertainty. Avoid blindly editing floating-point numbers or arbitrary memory.
5. Distinguish real-time UI/render/audio delta from simulation tick rate, time multiplier or tick accumulator. If a candidate also affects FMOD/camera, reject that hook and trace to a more specific simulation path.
6. Prototype read-only process discovery and inspection first; then implement reversible writes or narrow hooks only after verifying the exact installed binary version, expected bytes/types and callsite. No global Windows timers (QPC, GetTickCount, timeGetTime, Sleep) manipulation, no system-wide DLL hooks.
7. When speed > stock maximum, measure CPU and simulation throughput and cap/auto-throttle if the engine cannot keep up. No infinite loops, busy waits, disabling sync indiscriminately, or skipping updates.
8. Always provide reset to normal speed, automatic detach/revert where safely possible, crash-safe startup defaults, and refusal to patch on an unknown game build.
9. Work on a backed-up save. Smoke-test pause/resume, loading/reloading saves, construction, citizens, transport, audio, camera and shutdown.
10. Clearly label theoretical ideas as hypotheses and cite addresses/functions only after observing the real binary. Do not claim a patch works until locally reproduced.

## Engineering shape
- Language: Rust stable; Windows x64 target after executable verification.
- UI: `eframe/egui`; modular services, no god-file.
- Suggested modules: `process`, `pe_fingerprint`, `research`, `signatures`, `simulation_speed`, `safety`, `ui`.
- Commands: detect game, verify version, status, speed presets, restore 1x, collect diagnostic logs.
- Keep core logic independently testable; put Windows process access behind a trait/interface; use synthetic fixtures for unit tests.
- Document local manual setup and commands; respect error handling and run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` and a Windows release build when available.
- Never conceal a failed build or pretend to have tested on Windows if not done.

Read `docs/REVERSE_ENGINEERING_PLAN.md` and `research/RE_FINDINGS.md` for milestone requirements and measured evidence. The first valuable deliverable is an evidence-backed **simulation-speed call graph**, not an unverified high-speed slider.
