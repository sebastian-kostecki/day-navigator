"""One-off generator for the tray icons (32x32 RGBA).

Run from the repo root: py context/changes/source-health-status/make-tray-icons.py
States differ by shape as well as colour: check on green, exclamation mark on
red, dash on grey.
"""
from pathlib import Path

from PIL import Image, ImageDraw

SIZE = 32
OUT = Path("src-tauri/icons/tray")
WHITE = (255, 255, 255, 255)


def base(colour):
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(img).ellipse((1, 1, SIZE - 2, SIZE - 2), fill=colour)
    return img


def ok():
    img = base((34, 160, 76, 255))
    ImageDraw.Draw(img).line([(8, 17), (14, 23), (24, 10)], fill=WHITE, width=4, joint="curve")
    return img


def error():
    img = base((200, 40, 40, 255))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((14, 6, 17, 19), radius=1, fill=WHITE)
    d.ellipse((13, 22, 18, 27), fill=WHITE)
    return img


def idle():
    img = base((120, 120, 120, 255))
    ImageDraw.Draw(img).rounded_rectangle((8, 14, 23, 17), radius=1, fill=WHITE)
    return img


OUT.mkdir(parents=True, exist_ok=True)
for name, make in (("ok", ok), ("error", error), ("idle", idle)):
    make().save(OUT / f"{name}.png")
