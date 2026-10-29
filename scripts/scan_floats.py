#!/usr/bin/env python3
"""First pass over a decrypted code.bin, before Ghidra: find the float
constants a 3DS projection, viewport or HUD setup tends to load, and the
literal-pool words that point at them.

usage: scan_floats.py code.bin [--base 0x00100000]

code.bin is loaded at 0x00100000 for almost every title; check the text
address in exheader.bin if the numbers look off.
"""
import argparse
import struct

# value -> what it usually is
TARGETS = {
    400.0 / 240.0: "top-screen aspect 400/240",
    240.0 / 400.0: "inverse top-screen aspect",
    320.0 / 240.0: "bottom-screen aspect 320/240",
    400.0: "top-screen width",
    240.0: "screen height",
    320.0: "bottom-screen width",
    200.0: "half top width (HUD centre)",
    160.0: "half bottom width",
    120.0: "half height",
}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("code")
    ap.add_argument("--base", type=lambda s: int(s, 0), default=0x00100000)
    args = ap.parse_args()

    data = open(args.code, "rb").read()
    base = args.base
    words = {}
    for off in range(0, len(data) - 3, 4):
        words.setdefault(struct.unpack_from("<I", data, off)[0], []).append(off)

    for value, label in TARGETS.items():
        bits = struct.unpack("<I", struct.pack("<f", value))[0]
        hits = words.get(bits, [])
        print(f"{value:<12.6g} {bits:08X}  {label}: {len(hits)} hit(s)")
        for off in hits[:40]:
            addr = base + off
            # a word holding this address is a pointer to the constant,
            # usually from a literal pool next to the code that uses it
            refs = words.get(addr, [])
            ref_text = ", ".join(f"{base + r:08X}" for r in refs[:8])
            print(f"    {addr:08X}" + (f"  <- {ref_text}" if refs else ""))


if __name__ == "__main__":
    main()
