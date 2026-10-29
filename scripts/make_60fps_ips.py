#!/usr/bin/env python3
"""Writes code.ips, the 60 FPS patch for Kid Icarus: Uprising (USA,
0004000000030100), after checking that code.bin holds the original words.

usage: make_60fps_ips.py code.bin out/code.ips

The game runs its logic in steps of two vblanks and presents every other
one. The patch makes both every vblank:

  0x0048A7E4  mov r0,#30 -> #60     the frame rate the vsync manager is set to
  0x0048A7EC  mov r0,#2  -> #1      its swap interval, vblanks per present
  0x0091041C  and r2,r2,#1 -> mov r2,#0
                                    presenting kept frames on even vblanks,
                                    which at interval 1 made every third wait
  0x00828E8C  add r6,r1,r0,lsr #1 -> add r6,r1,r0
                                    updates to run: vblanks over two -> vblanks
  0x00828E30  2.0 -> 1.0            vblanks a step covers, so time steps 1/60

When a frame runs long the game already runs extra updates to catch up, so
it keeps its speed below 60.
"""
import struct
import sys

BASE = 0x00100000
PATCHES = [
    (0x0048A7E4, 0xE3A0001E, 0xE3A0003C),
    (0x0048A7EC, 0xE3A00002, 0xE3A00001),
    (0x0091041C, 0xE2022001, 0xE3A02000),
    (0x00828E8C, 0xE08160A0, 0xE0816000),
    (0x00828E30, 0x40000000, 0x3F800000),
]


def main():
    code = open(sys.argv[1], "rb").read()
    records = b""
    for address, original, patched in PATCHES:
        offset = address - BASE
        found, = struct.unpack_from("<I", code, offset)
        if found != original:
            sys.exit(f"{address:08X} holds {found:08X}, not {original:08X}: a different build or already patched")
        records += offset.to_bytes(3, "big") + (4).to_bytes(2, "big") + struct.pack("<I", patched)
    open(sys.argv[2], "wb").write(b"PATCH" + records + b"EOF")
    print(f"wrote {sys.argv[2]}, {len(PATCHES)} changes")


if __name__ == "__main__":
    main()
