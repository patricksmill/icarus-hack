#!/usr/bin/env python3
"""Rewrites Ghidra's DAT_xxxxxxxx references in a decompile dump with the
word they point at, when that word looks like an ordinary float, so
constants such as -1.0 or 0.5 can be grepped for:

    DAT_0063c078  ->  DAT_0063c078/*1.66667*/

usage: annotate_floats.py code.bin decomp.c out.c [--base 0x00100000]
"""
import argparse
import math
import re
import struct


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("code")
    ap.add_argument("decomp")
    ap.add_argument("out")
    ap.add_argument("--base", type=lambda s: int(s, 0), default=0x00100000)
    args = ap.parse_args()
    code = open(args.code, "rb").read()
    cache = {}

    def value(addr):
        if addr in cache:
            return cache[addr]
        off = addr - args.base
        text = None
        if 0 <= off <= len(code) - 4 and off % 4 == 0:
            f, = struct.unpack_from("<f", code, off)
            # skip pointers, small integers and other non-float words
            if f == 0.0 or (math.isfinite(f) and 1e-6 < abs(f) < 1e7):
                text = f"{f:.6g}"
        cache[addr] = text
        return text

    def sub(m):
        v = value(int(m.group(1), 16))
        return m.group(0) + (f"/*{v}*/" if v else "")

    pattern = re.compile(r"\bDAT_([0-9a-f]{8})\b(?!/\*)")
    with open(args.decomp) as src, open(args.out, "w") as dst:
        for line in src:
            dst.write(pattern.sub(sub, line))


if __name__ == "__main__":
    main()
