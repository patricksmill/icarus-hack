#!/usr/bin/env python3
"""Lists the changed byte ranges between two code.bin files, merging ranges
closer than --gap bytes, with the address each starts at.

usage: diff_code.py old.bin new.bin [--base 0x00100000] [--gap 16]
"""
import argparse


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("old")
    ap.add_argument("new")
    ap.add_argument("--base", type=lambda s: int(s, 0), default=0x00100000)
    ap.add_argument("--gap", type=int, default=16)
    args = ap.parse_args()
    a = open(args.old, "rb").read()
    b = open(args.new, "rb").read()
    n = min(len(a), len(b))
    ranges = []
    i = 0
    while i < n:
        if a[i] != b[i]:
            start = i
            last = i
            while i < n and i - last <= args.gap:
                if a[i] != b[i]:
                    last = i
                i += 1
            ranges.append((start, last + 1))
        else:
            i += 1
    for start, end in ranges:
        print(f"{args.base + start:08X} {end - start:6d}  {a[start:min(end, start + 16)].hex()} -> {b[start:min(end, start + 16)].hex()}")
    if len(a) != len(b):
        print(f"sizes differ: {len(a):#x} -> {len(b):#x}, extra {len(b) - n:#x} bytes from {args.base + n:08X}")
    print(f"{len(ranges)} ranges, {sum(e - s for s, e in ranges)} bytes")


if __name__ == "__main__":
    main()
