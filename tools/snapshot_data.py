"""Read-only .data snapshot of the verified local Ostriv build.

Usage: python tools/snapshot_data.py PID MODULE_BASE_HEX GAME_EXE OUTPUT
Get MODULE_BASE_HEX from (Get-Process ostriv).MainModule.BaseAddress.
Outputs are private and must stay under research/local/.
"""

import argparse
import ctypes
import hashlib
import pathlib
import struct

EXPECTED_SHA256 = "d07fdcde38cc4a1e57696332b68ff6f74bf8fb931659b71b414d28c1a89d8be0"
PROCESS_VM_READ = 0x0010
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("module_base_hex", type=lambda s: int(s, 16))
    parser.add_argument("game_exe", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()
    exe = args.game_exe.read_bytes()
    if hashlib.sha256(exe).hexdigest() != EXPECTED_SHA256:
        raise SystemExit("Installed game hash differs from the researched build")
    pe = struct.unpack_from("<I", exe, 0x3C)[0]
    count = struct.unpack_from("<H", exe, pe + 6)[0]
    optional_size = struct.unpack_from("<H", exe, pe + 20)[0]
    sections = pe + 24 + optional_size
    data_section = None
    for index in range(count):
        off = sections + index * 40
        if exe[off:off + 8].rstrip(b"\0") == b".data":
            data_section = struct.unpack_from("<II", exe, off + 8)
            break
    if data_section is None:
        raise SystemExit(".data section not found")
    data_size, data_rva = data_section
    if args.output.parent.resolve() != pathlib.Path("research/local").resolve():
        raise SystemExit("Output must be directly inside research/local")
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.OpenProcess.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
    kernel.OpenProcess.restype = ctypes.c_void_p
    kernel.ReadProcessMemory.argtypes = [
        ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
        ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t),
    ]
    kernel.ReadProcessMemory.restype = ctypes.c_int
    kernel.CloseHandle.argtypes = [ctypes.c_void_p]
    handle = kernel.OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_LIMITED_INFORMATION, 0, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess failed")
    try:
        data = ctypes.create_string_buffer(data_size)
        read = ctypes.c_size_t()
        ok = kernel.ReadProcessMemory(handle, args.module_base_hex + data_rva, data, data_size, ctypes.byref(read))
        if not ok or read.value != data_size:
            raise OSError(ctypes.get_last_error(), f"ReadProcessMemory read {read.value}/{data_size}")
        args.output.write_bytes(data.raw)
        print(f"Saved {read.value} bytes from .data (RVA {data_rva:#x})")
    finally:
        kernel.CloseHandle(handle)


if __name__ == "__main__":
    main()
