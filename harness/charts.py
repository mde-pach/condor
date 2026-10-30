"""Draw the benchmark charts from harness/benchmark.json (written by bench.py).

Usage: python harness/charts.py

Writes docs/assets/bench-{dark,light}.svg (time to scan one large project,
condor vs vulture) and docs/assets/speedup-{dark,light}.svg (how many times
faster condor is on each input). Static SVG, so it renders in the README and
the docs.
"""

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE.parent / "docs" / "assets"
HEADLINE = "django"
FONT = "Inter, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif"
MONO = "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace"
THEMES = {
    # Text uses ink tokens; condor's bar carries the brand amber, vulture's a neutral.
    "dark": {"ink": "#E6E1D6", "ink2": "#8A8F98", "grid": "#222832", "condor": "#F2A541", "other": "#6E7681"},
    "light": {"ink": "#1A1D23", "ink2": "#5C626B", "grid": "#DDD8CD", "condor": "#C77A12", "other": "#7D8794"},
}


def bar(x, y, w, h, fill, r=4):
    """A bar anchored at x with a rounded data end."""
    w = max(w, r)
    return (
        f'<path d="M{x} {y} H{x + w - r} Q{x + w} {y} {x + w} {y + r} V{y + h - r} '
        f'Q{x + w} {y + h} {x + w - r} {y + h} H{x} Z" fill="{fill}"/>'
    )


def fmt_seconds(s):
    return f"{s * 1000:.0f} ms" if s < 1 else f"{s:.2f} s"


def headline(data, c):
    run = next(r for r in data["runs"] if r["input"] == HEADLINE)
    rows = [("condor", run["condor_s"], c["condor"]), ("vulture", run["vulture_s"], c["other"])]
    width, left, right, top, row_h, bar_h = 720, 96, 96, 56, 44, 24
    plot = width - left - right
    scale_max = max(v for _, v, _ in rows)
    ticks = [t for t in range(0, int(scale_max) + 2) if t <= scale_max * 1.05]
    height = top + row_h * len(rows) + 40
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" '
        f'role="img" aria-label="Scanning Django ({run["files"]:,} files): condor {fmt_seconds(run["condor_s"])}, '
        f'vulture {fmt_seconds(run["vulture_s"])}">',
        f'<text x="{left}" y="24" font-family="{FONT}" font-size="15" font-weight="600" fill="{c["ink"]}">'
        f'Scanning Django ({run["files"]:,} files)</text>',
    ]
    axis_y = top + row_h * len(rows)
    for t in ticks:
        x = left + plot * t / scale_max
        out.append(f'<line x1="{x:.1f}" y1="{top - 8}" x2="{x:.1f}" y2="{axis_y}" stroke="{c["grid"]}" stroke-width="1"/>')
        out.append(
            f'<text x="{x:.1f}" y="{axis_y + 18}" font-family="{FONT}" font-size="12" fill="{c["ink2"]}" '
            f'text-anchor="middle">{t} s</text>'
        )
    for i, (name, value, color) in enumerate(rows):
        y = top + i * row_h + (row_h - bar_h) / 2
        w = plot * value / scale_max
        out.append(
            f'<text x="{left - 12}" y="{y + bar_h / 2 + 5:.1f}" font-family="{MONO}" font-size="14" '
            f'font-weight="{700 if name == "condor" else 400}" fill="{c["ink"]}" text-anchor="end">{name}</text>'
        )
        out.append(bar(left, y, w, bar_h, color))
        out.append(
            f'<text x="{left + w + 8:.1f}" y="{y + bar_h / 2 + 5:.1f}" font-family="{FONT}" font-size="13" '
            f'fill="{c["ink"]}">{fmt_seconds(value)}</text>'
        )
    out.append("</svg>")
    return "\n".join(out)


def speedups(data, c):
    runs = data["runs"]
    width, left, right, top, row_h, bar_h = 720, 150, 70, 56, 30, 16
    plot = width - left - right
    factors = [r["vulture_s"] / r["condor_s"] for r in runs]
    scale_max = max(factors)
    height = top + row_h * len(runs) + 12
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" '
        f'role="img" aria-label="How many times faster condor is than vulture on each input">',
        f'<text x="{left}" y="24" font-family="{FONT}" font-size="15" font-weight="600" fill="{c["ink"]}">'
        "How many times faster than vulture</text>",
    ]
    for i, (r, f) in enumerate(zip(runs, factors)):
        y = top + i * row_h + (row_h - bar_h) / 2
        w = plot * f / scale_max
        label = f'{r["input"]} ({r["files"]:,} files)' if r["files"] > 1 else r["input"]
        out.append(
            f'<text x="{left - 12}" y="{y + bar_h / 2 + 4.5:.1f}" font-family="{FONT}" font-size="13" '
            f'fill="{c["ink2"]}" text-anchor="end">{label}</text>'
        )
        out.append(bar(left, y, w, bar_h, c["condor"]))
        out.append(
            f'<text x="{left + w + 8:.1f}" y="{y + bar_h / 2 + 4.5:.1f}" font-family="{FONT}" font-size="13" '
            f'font-weight="600" fill="{c["ink"]}">{f:.0f}×</text>'
        )
    out.append("</svg>")
    return "\n".join(out)


def main():
    data = json.loads((HERE / "benchmark.json").read_text())
    OUT.mkdir(parents=True, exist_ok=True)
    for theme, colors in THEMES.items():
        (OUT / f"bench-{theme}.svg").write_text(headline(data, colors) + "\n")
        (OUT / f"speedup-{theme}.svg").write_text(speedups(data, colors) + "\n")
    print(f"charts written to {OUT}")


if __name__ == "__main__":
    main()
