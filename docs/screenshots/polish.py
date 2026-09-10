#!/usr/bin/env python3
"""Crop, round, shadow and downscale raw app screenshots for the README.

Usage:
    1. Capture each screen (macOS: Cmd+Shift+4, then Space, then click the window).
    2. Drop the raw PNGs in this folder named `_raw-<name>.png`.
    3. Adjust the crop box for each in JOBS below (left, top, right, bottom, in raw px).
    4. `python3 docs/screenshots/polish.py`  (needs Pillow: `pip install --user Pillow`)

Outputs `<name>.png` — rounded corners + soft shadow on a transparent margin, so it
sits well on both light and dark GitHub themes. Raw files are left untouched.
"""

import os

from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))

# name: (raw file, (left, top, right, bottom) crop in raw px, output width px)
JOBS = {
    "hero": ("_raw-hero.png", (42, 28, 3060, 1583), 2400),
    "dashboard": ("_raw-dashboard.png", (16, 2, 2258, 1620), 1800),
    "create": ("_raw-create.png", (30, 16, 2262, 1642), 1800),
    "dependencies": ("_raw-dependencies.png", (20, 14, 2258, 1628), 1800),
    "profiles": ("_raw-profiles.png", (26, 16, 2260, 1622), 1800),
}


def polish(src, box, out_width, out_name):
    im = Image.open(os.path.join(HERE, src)).convert("RGB").crop(box)
    w, h = im.size
    im = im.resize((out_width, round(h * out_width / w)), Image.LANCZOS)

    radius = round(out_width * 0.010)
    mask = Image.new("L", im.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, im.size[0] - 1, im.size[1] - 1], radius=radius, fill=255
    )
    card = Image.new("RGBA", im.size, (0, 0, 0, 0))
    card.paste(im, (0, 0), mask)

    margin = round(out_width * 0.026)
    canvas = Image.new("RGBA", (im.size[0] + 2 * margin, im.size[1] + 2 * margin), (0, 0, 0, 0))
    shadow = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        [margin, margin + round(margin * 0.35),
         margin + im.size[0], margin + im.size[1] + round(margin * 0.35)],
        radius=radius, fill=(0, 0, 0, 115),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(round(margin * 0.55)))
    canvas = Image.alpha_composite(canvas, shadow)
    canvas.alpha_composite(card, (margin, margin))
    canvas.save(os.path.join(HERE, out_name), optimize=True)
    print(f"{out_name:16s} {canvas.size[0]}x{canvas.size[1]}")


if __name__ == "__main__":
    for name, (src, box, width) in JOBS.items():
        polish(src, box, width, name + ".png")
