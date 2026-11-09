"""Check the actual semantic token pairs used for readable studio text."""
import re
from pathlib import Path

source = (Path(__file__).resolve().parents[1] / "crates/studio/src/theme.rs").read_text(encoding="utf-8")
tokens = {
    name: tuple(map(int, rgb))
    for name, *rgb in re.findall(
        r"pub const (\w+): Color = Color::from_rgb8\((\d+), (\d+), (\d+)\)", source
    )
}

def luminance(rgb):
    channels = [v / 255 for v in rgb]
    linear = [v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4 for v in channels]
    return sum(v * w for v, w in zip(linear, (0.2126, 0.7152, 0.0722)))

pairs = [("TEXT", "SURFACE"), ("MUTED", "SURFACE"), ("MUTED", "HOVER"), ("ACCENT", "SURFACE"), ("INSET", "ACCENT"), ("SIGNAL", "SURFACE"), ("WARNING", "SURFACE")]
for foreground, background in pairs:
    values = sorted([luminance(tokens[foreground]), luminance(tokens[background])])
    ratio = (values[1] + 0.05) / (values[0] + 0.05)
    print(f"{foreground:>8} / {background:<10} {ratio:.2f}:1")
    if ratio < 4.5:
        raise SystemExit("Text contrast check failed")
