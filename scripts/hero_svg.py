"""Generate the README hero: an animated SVG of "condor" losing its two unused o's.

Usage: uv run --with fonttools==4.55.3 python scripts/hero_svg.py FONT_DIR

FONT_DIR holds JetBrainsMono-ExtraBold.ttf and JetBrainsMono-Regular.ttf
(https://github.com/JetBrains/JetBrainsMono, OFL). Text is converted to paths
so the image looks the same on GitHub, where web fonts do not load. Writes
docs/assets/hero-dark.svg and docs/assets/hero-light.svg. The sequence is the
docs hero's (docs/stylesheets/condor.css), looping every 7 s.
"""

import sys
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parent.parent
THEMES = {
    "dark": {"fg": "#E6E1D6", "muted": "#8A8F98", "amber": "#F2A541", "gold": "#E0A84B"},
    "light": {"fg": "#1A1D23", "muted": "#5C626B", "amber": "#A35F00", "gold": "#A97319"},
}
# The D1 mark (viewBox 0 0 200 200).
MARK = (
    '<g transform="translate(10.5 2)" fill="none" stroke="{gold}" stroke-width="10" '
    'stroke-linejoin="round" stroke-linecap="round"><path d="M150 170 C156 140 158 110 150 88 '
    "C142 66 124 56 104 58 C108 52 110 46 106 40 C100 30 82 26 66 32 C46 40 40 66 47.5 86.2 "
    "C42.6 88.2 38 90.8 34 94 C24 102 21 116 25 128 C27 134 31 138 36 140 C35 131 37 123 44 119 "
    'C52 115 62 115 72 117 C86 124 94 146 96 170"/></g><circle cx="122.5" cy="90" r="8" fill="{gold}"/>'
)


class Font:
    def __init__(self, path):
        self.font = TTFont(path)
        self.glyphs = self.font.getGlyphSet()
        self.cmap = self.font.getBestCmap()
        self.upem = self.font["head"].unitsPerEm

    def path(self, text, x, baseline, size):
        """SVG path data for `text` set at (x, baseline), and the advance."""
        scale = size / self.upem
        pen = SVGPathPen(self.glyphs)
        for ch in text:
            name = self.cmap[ord(ch)]
            glyph = self.glyphs[name]
            glyph.draw(TransformPen(pen, (scale, 0, 0, -scale, x, baseline)))
            x += glyph.width * scale
        return pen.getCommands(), x


def wave(x, y, width, amplitude=3.0, period=12.0):
    """A wavy underline from x to x + width."""
    d = f"M{x:.1f} {y:.1f}"
    steps = int(width // (period / 2))
    for i in range(steps):
        cx = x + (i + 0.5) * period / 2
        cy = y + (amplitude if i % 2 else -amplitude)
        d += f" Q{cx:.1f} {cy:.1f} {x + (i + 1) * period / 2:.1f} {y:.1f}"
    return d


def build(bold, regular, theme):
    c = THEMES[theme]
    size, x0, base = 112, 150, 100
    adv = 0.6 * size  # monospace advance
    parts = {}
    for key, text, i in [("c", "c", 0), ("o1", "o", 1), ("nd", "nd", 2), ("o2", "o", 4), ("r", "r", 5)]:
        parts[key], _ = bold.path(text, x0 + i * adv, base, size)

    def line(prefix, y):
        small = 15
        d_pre, x = regular.path(prefix, x0, y, small)
        d_hit, x = regular.path("unused letter 'o'", x, y, small)
        d_post, _ = regular.path(" (100% confidence)", x, y, small)
        return (
            f'<path d="{d_pre}" fill="{c["muted"]}"/><path d="{d_hit}" fill="{c["amber"]}"/>'
            f'<path d="{d_post}" fill="{c["muted"]}"/>'
        )

    waves = "".join(
        f'<path class="wave" d="{wave(x0 + i * adv + 4, base + 16, adv - 8)}" fill="none" '
        f'stroke="{c["amber"]}" stroke-width="3" stroke-linecap="round"/>'
        for i in (1, 4)
    )
    # 7 s loop: 1.0 s flag, 1.2 / 1.5 s report, 3.2 s vanish, 3.25-3.5 s close up, hold.
    style = f"""
.o {{ animation: flag 7s step-end infinite; }}
.wave {{ opacity: 0; animation: show-flag 7s step-end infinite; }}
.l1 {{ opacity: 0; animation: l1 7s step-end infinite; }}
.l2 {{ opacity: 0; animation: l2 7s step-end infinite; }}
.nd {{ animation: shift1 7s ease-in-out infinite; }}
.r {{ animation: shift2 7s ease-in-out infinite; }}
@keyframes flag {{ 0% {{ fill: {c["fg"]}; opacity: 1; }} 14.29% {{ fill: {c["amber"]}; opacity: 1; }} 45.71%, 100% {{ fill: {c["amber"]}; opacity: 0; }} }}
@keyframes show-flag {{ 0% {{ opacity: 0; }} 14.29% {{ opacity: 1; }} 45.71%, 100% {{ opacity: 0; }} }}
@keyframes l1 {{ 0% {{ opacity: 0; }} 17.14% {{ opacity: 1; }} 45.71%, 100% {{ opacity: 0; }} }}
@keyframes l2 {{ 0% {{ opacity: 0; }} 21.43% {{ opacity: 1; }} 45.71%, 100% {{ opacity: 0; }} }}
@keyframes shift1 {{ 0%, 46.43% {{ transform: translateX(0); }} 50%, 100% {{ transform: translateX(-{adv:.1f}px); }} }}
@keyframes shift2 {{ 0%, 46.43% {{ transform: translateX(0); }} 50%, 100% {{ transform: translateX(-{2 * adv:.1f}px); }} }}
@media (prefers-reduced-motion: reduce) {{
  .o, .wave, .l1, .l2 {{ animation: none; opacity: 0; }}
  .nd {{ animation: none; transform: translateX(-{adv:.1f}px); }}
  .r {{ animation: none; transform: translateX(-{2 * adv:.1f}px); }}
}}"""
    fg = c["fg"]
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="600" height="180" viewBox="0 0 600 180" role="img" aria-label="condor: its two unused letters o are flagged and removed, leaving cndr">
<style>{style}
</style>
<g transform="translate(0 4) scale(0.6)">{MARK.format(gold=c["gold"])}</g>
<path d="{parts["c"]}" fill="{fg}"/>
<path class="o" d="{parts["o1"]}" fill="{fg}"/>
<path class="nd" d="{parts["nd"]}" fill="{fg}"/>
<path class="o" d="{parts["o2"]}" fill="{fg}" style="transform-box: view-box"/>
<path class="r" d="{parts["r"]}" fill="{fg}"/>
{waves}
<g class="l1">{line("condor:1:2: ", 142)}</g>
<g class="l2">{line("condor:1:5: ", 164)}</g>
</svg>
"""


def main():
    fonts = Path(sys.argv[1])
    bold = Font(fonts / "JetBrainsMono-ExtraBold.ttf")
    regular = Font(fonts / "JetBrainsMono-Regular.ttf")
    out = ROOT / "docs" / "assets"
    out.mkdir(parents=True, exist_ok=True)
    for theme in THEMES:
        (out / f"hero-{theme}.svg").write_text(build(bold, regular, theme))
        print(out / f"hero-{theme}.svg")


if __name__ == "__main__":
    main()
