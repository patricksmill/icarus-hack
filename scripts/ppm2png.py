#!/usr/bin/env python3
"""Converts P6 .ppm screenshots to .png next to them; with two files per
frame (_top, _bottom) also writes _both.png with the screens stacked.

usage: ppm2png.py file.ppm...
"""
import os
import struct
import sys
import zlib


def read_ppm(path):
    data = open(path, "rb").read()
    parts = data.split(b"\n", 3)
    w, h = map(int, parts[1].split())
    return w, h, parts[3]


def write_png(path, w, h, rgb):
    raw = b"".join(b"\0" + rgb[y * w * 3:(y + 1) * w * 3] for y in range(h))

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


def main():
    for path in sys.argv[1:]:
        w, h, rgb = read_ppm(path)
        write_png(path[:-4] + ".png", w, h, rgb)
        if path.endswith("_top.ppm") and os.path.exists(path.replace("_top", "_bottom")):
            bw, bh, brgb = read_ppm(path.replace("_top", "_bottom"))
            pad = (w - bw) // 2
            rows = [rgb[y * w * 3:(y + 1) * w * 3] for y in range(h)]
            rows += [b"\0" * pad * 3 + brgb[y * bw * 3:(y + 1) * bw * 3] + b"\0" * (w - bw - pad) * 3 for y in range(bh)]
            write_png(path.replace("_top.ppm", "_both.png"), w, h + bh, b"".join(rows))


if __name__ == "__main__":
    main()
