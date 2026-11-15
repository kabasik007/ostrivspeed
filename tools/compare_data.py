"""List .data dwords that transition with stock presets in three snapshots.

Usage: python tools/compare_data.py research/local/one.bin research/local/five.bin research/local/twelve.bin
Candidate values are clues only, not proof of a setter or a safe patch.
"""

import argparse
import pathlib
import struct

DATA_RVA = 0x64F000


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("one", type=pathlib.Path)
    parser.add_argument("five", type=pathlib.Path)
    parser.add_argument("twelve", type=pathlib.Path)
    args = parser.parse_args()
    buffers = [path.read_bytes() for path in (args.one, args.five, args.twelve)]
    if len(set(map(len, buffers))) != 1:
        raise SystemExit("Snapshots have unequal sizes")
    matches = 0
    for offset in range(0, len(buffers[0]) - 7, 4):
        words = [struct.unpack_from("<I", b, offset)[0] for b in buffers]
        floats = [struct.unpack_from("<f", b, offset)[0] for b in buffers]
        doubles = [struct.unpack_from("<d", b, offset)[0] for b in buffers]
        if words in ([1, 5, 12], [0, 1, 2], [1, 2, 3]) or floats == [1.0, 5.0, 12.0] or doubles == [1.0, 5.0, 12.0]:
            print(f"RVA {DATA_RVA + offset:#x}: int={words} float={floats} double={doubles}")
            matches += 1
    print(f"{matches} value-pattern candidates; verify with debugger xrefs before any write")


if __name__ == "__main__":
    main()
