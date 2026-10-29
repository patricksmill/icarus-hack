#!/usr/bin/env python3
"""Contact sheet of screenshot pairs <prefix><n>_top.ppm/_bottom.ppm at
half size, 4 per row.

usage: sheet.py dir prefix count out.png
"""
import struct
import sys
import zlib


def rd(p):
    d = open(p, "rb").read()
    parts = d.split(b"\n", 3)
    w, h = map(int, parts[1].split())
    return w, h, parts[3]


def wr(path, w, h, rgb):
    raw = b"".join(b"\0" + rgb[y * w * 3:(y + 1) * w * 3] for y in range(h))
    ch = lambda k, b: struct.pack(">I", len(b)) + k + b + struct.pack(">I", zlib.crc32(k + b))
    open(path, "wb").write(b"\x89PNG\r\n\x1a\n" + ch(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                           + ch(b"IDAT", zlib.compress(raw)) + ch(b"IEND", b""))


def main():
    d, prefix, count, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    W, H, G, cols = 200, 240, 4, 4
    imgs = []
    for i in range(1, count + 1):
        w, h, t = rd(f"{d}/{prefix}{i}_top.ppm")
        bw, bh, b = rd(f"{d}/{prefix}{i}_bottom.ppm")
        img = bytearray(W * H * 3)
        for y in range(120):
            for x in range(200):
                s = (y * 2 * w + x * 2) * 3
                img[(y * W + x) * 3:(y * W + x) * 3 + 3] = t[s:s + 3]
            for x in range(160):
                s = (y * 2 * bw + x * 2) * 3
                o = ((120 + y) * W + x + 20) * 3
                img[o:o + 3] = b[s:s + 3]
        imgs.append(bytes(img))
    rows = (len(imgs) + cols - 1) // cols
    SW, SH = cols * W + (cols - 1) * G, rows * H + (rows - 1) * G
    sheet = bytearray(b"\x40" * SW * SH * 3)
    for k, img in enumerate(imgs):
        r, c = divmod(k, cols)
        ox, oy = c * (W + G), r * (H + G)
        for y in range(H):
            sheet[((oy + y) * SW + ox) * 3:((oy + y) * SW + ox + W) * 3] = img[y * W * 3:(y + 1) * W * 3]
    wr(out, SW, SH, bytes(sheet))


if __name__ == "__main__":
    main()
