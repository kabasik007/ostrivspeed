# Reverse engineering findings — installed Ostriv build

## Artifact identity

- Steam build ID: `26211342` (local `appmanifest_773790.acf`).
- In-game version text: `Alpha 5 patch 9 (ver. 0.5.9.65)`.
- `ostriv.exe`: PE32+, AMD64, 7,499,264 bytes, SHA-256 `d07fdcde38cc4a1e57696332b68ff6f74bf8fb931659b71b414d28c1a89d8be0`.
- Binary Ninja Free 6.0.10905 is installed. Its headless Python API is unavailable here; disassembly below was obtained with locally installed Visual Studio `dumpbin /DISASM`. The game binary was never edited. Private disassembly and RAM snapshots are ignored under `research/local/`.

## Native speed path (this hash only)

All addresses below are module-relative RVAs, **not** ASLR runtime addresses. The preferred PE image base is `0x140000000`.

| RVA | Observed role | Evidence |
|---|---|---|
| `0x64F860` | Target speed, `double` | Read-only RAM snapshots at stock 1×/5×/12× held exactly `1.0`/`5.0`/`12.0`; returned to `1.0` on a repeat 1× snapshot. Initial on-disk value `1.0`. |
| `0x64F160` | Current speed, `double` | Same stock-preset transitions; frame update reads target and writes this value with smoothing logic. |
| `0x64F868` | Speed coefficient, `float` | `1.0` in all snapshots; loaded and multiplied into the time delta at RVA `0x264367`. Its configuration provenance needs further tracing. |
| `0x64F170` | Accumulated world time, `double` | Increases in live RAM; updated at RVA `0x26438E`, read by many world-time consumers. Units inferred to be roughly one per real second at 1× in the 10-second baseline. |
| `0x675910` | Per-update scaled delta, `double` | Written at RVA `0x264386` after multiplying the engine delta by current speed and the coefficient. Its consumers still need subsystem labeling. |
| `0x3F4A00`, `0x3F4A50`, `0x3F4AA0` | UI preset callback bodies | Load constant `12.0`, `5.0`, `1.0` respectively and store to `0x64F860`. Static disassembly. |
| `0x286CE5` | Keyboard/input path into target speed | Stores a preset constant to `0x64F860` after key/state checks. Other branches need annotation. |
| `0x2642F0` | Speed/time update | Reads world time, current and target speed and real delta; updates per-frame scaled delta, world time and current speed. Called at RVA `0x31FC20` only when three state flags permit; alternate path at `0x31FC27` resets time-related fields. |

Observed path:

```text
stock UI buttons / input handling
  -> 0x3F4A00 / 0x3F4A50 / 0x3F4AA0 (or input branch 0x286CE5)
  -> target double 0x64F860
  -> update function 0x2642F0, called from 0x31FC20
  -> current double 0x64F160
  -> engine delta × current speed × coefficient 0x64F868
  -> scaled delta 0x675910 + accumulated world time 0x64F170
  -> world-time consumers (not yet exhaustively classified)
```

The installed binary contains labels `Game speed x1`, `Game speed x5`, `Game speed x12` and `Scale sounds to game speed`. The last is an option label, **not** proof that audio is independent at every accelerated setting. Static `0x64F160` readers include RVAs `0x78F4B`, `0x791F9`, `0x82ACE`, `0xDD80D`; their subsystem roles require further analysis.

## Dynamic observations and limits

- The live game displayed the same three stock speed buttons. User-controlled preset changes produced matching `double` transitions in both variables. No RAM write was used to discover these addresses.
- Two 15-second experimental writes to target `0x64F860` read back `15.0`, then read back `1.0` after automatic restore. The user observed faster world progression with normal music and camera. This is one live scenario, not a full regression across saves, construction, audio types and camera controls.
- One later 15× timed sample produced zero world-time delta and current speed `1.0`, while target readback was 15×. The game appeared paused; this sample does not measure 15× throughput.
- In an active scenario, eight-second samples yielded effective world-time rates of `14.37×`, `18.87×`, `27.32×`, and `42.47×` for requested 15×, 20×, 30×, and 50×. The user reported normal music and camera at 15× and 50×. These short intervals do not establish construction/production throughput or long-run stability.
- Requested 60× yielded only `0.13×` effective over eight seconds, with current speed reading `0` at the end. The user reports the game stopped or hung without manual pause, then recovered. The controller now refuses 60×.
- No debugger watchpoint/call-stack capture has yet been completed. The call graph is based on static instructions plus controlled RAM values. Exact subsystem isolation and long-run stability remain under test.

## Reproduction

1. Check the game SHA-256 against the identity above.
2. With a running game, obtain PID and module base from `target/debug/ostrivspeed.exe --status` or PowerShell `Get-Process ostriv`.
3. Use `tools/snapshot_data.py` at stock 1×, 5× and 12×; save snapshots only under ignored `research/local/`.
4. Run `tools/compare_data.py` on the three files. It reports only RVAs `0x64F160` and `0x64F860` with the `double` sequence `1.0, 5.0, 12.0` for the observed session.
5. Inspect xrefs in a disassembler and verify the specific setter and time-update instructions before considering another game build.
