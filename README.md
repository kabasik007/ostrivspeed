# OstrivSpeed

**Native simulation-speed research and Rust controller for Ostriv (Windows, single-player).**

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

**Status:** research scaffold only. No game-specific addresses, successful patches, or verified 50x acceleration have been identified.
