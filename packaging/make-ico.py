#!/usr/bin/env python3
"""Convert a square PNG into a multi-resolution Windows .ico.

Usage: make-ico.py <input.png> <output.ico>
Requires Pillow (`pip install pillow`).
"""
import sys

from PIL import Image

SIZES = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit("usage: make-ico.py <input.png> <output.ico>")
    src, dst = sys.argv[1], sys.argv[2]
    img = Image.open(src).convert("RGBA")
    img.save(dst, format="ICO", sizes=SIZES)
    print(f"wrote {dst}")


if __name__ == "__main__":
    main()
