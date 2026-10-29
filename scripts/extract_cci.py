#!/usr/bin/env python3
"""Pulls exheader.bin, the ExeFS files and the RomFS out of the first
partition of a decrypted CCI/.3ds. code.bin comes out decompressed when the
exheader says it is compressed.

usage: extract_cci.py game.cci outdir
"""
import os
import struct
import sys

MEDIA = 0x200


def blz_decompress(data):
    """Nintendo's backwards LZ77, as used for a compressed .code."""
    buf_top, = struct.unpack_from("<I", data, len(data) - 8)
    extra, = struct.unpack_from("<I", data, len(data) - 4)
    enc_size = buf_top & 0xFFFFFF
    header = buf_top >> 24
    out = bytearray(data) + bytearray(extra)
    src = len(data) - header
    dst = len(out)
    end = len(data) - enc_size
    while src > end:
        src -= 1
        flags = out[src]
        for _ in range(8):
            if src <= end:
                break
            if flags & 0x80:
                src -= 2
                pair = out[src] | out[src + 1] << 8
                length = (pair >> 12) + 3
                disp = (pair & 0xFFF) + 3
                for _ in range(length):
                    dst -= 1
                    out[dst] = out[dst + disp]
            else:
                src -= 1
                dst -= 1
                out[dst] = out[src]
            flags = (flags << 1) & 0xFF
    return bytes(out)


def main():
    path, outdir = sys.argv[1], sys.argv[2]
    f = open(path, "rb")
    ncsd = f.read(0x200)
    assert ncsd[0x100:0x104] == b"NCSD", "not a CCI"
    part, = struct.unpack_from("<I", ncsd, 0x120)
    base = part * MEDIA
    f.seek(base)
    ncch = f.read(0x200)
    assert ncch[0x100:0x104] == b"NCCH"
    assert ncch[0x18F] & 0x04, "partition is encrypted"

    os.makedirs(os.path.join(outdir, "exefs"), exist_ok=True)
    exh_size, = struct.unpack_from("<I", ncch, 0x180)
    f.seek(base + 0x200)
    exheader = f.read(0x800)
    open(os.path.join(outdir, "exheader.bin"), "wb").write(exheader)
    compressed = exheader[0xD] & 1

    exefs_off, exefs_size = struct.unpack_from("<II", ncch, 0x1A0)
    f.seek(base + exefs_off * MEDIA)
    exefs = f.read(exefs_size * MEDIA)
    for i in range(10):
        name, off, size = struct.unpack_from("<8sII", exefs, i * 16)
        name = name.rstrip(b"\0").decode()
        if not size:
            continue
        blob = exefs[0x200 + off:0x200 + off + size]
        if name == ".code" and compressed:
            blob = blz_decompress(blob)
        fname = "code.bin" if name == ".code" else name + ".bin"
        open(os.path.join(outdir, "exefs", fname), "wb").write(blob)
        print(f"exefs/{fname}: {len(blob):#x} bytes")

    romfs_off, romfs_size = struct.unpack_from("<II", ncch, 0x1B0)
    print(f"romfs at {base + romfs_off * MEDIA:#x}, {romfs_size * MEDIA:#x} bytes")

    # code set info: text, rodata and data address / pages / size
    for i, name in enumerate(("text", "rodata", "data")):
        addr, pages, size = struct.unpack_from("<III", exheader, 0x10 + i * 0x10)
        print(f"{name}: {addr:#010x} size {size:#x}")
    bss, = struct.unpack_from("<I", exheader, 0x3C)
    print(f"bss size {bss:#x}, code {'compressed' if compressed else 'plain'}")


if __name__ == "__main__":
    main()
