"""Write harness/SUMMARY.md: a short, readable comparison per project.

Usage: python harness/summarize.py   (difftest.py runs it after each run)

Reads harness/results.json and harness/corpus.toml. Only the default-flags
runs on real projects are summarised; harness/REPORT.md has every run.
"""

import json
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent


def change(before, after):
    if before == 0:
        return "—"
    pct = round((after - before) * 100 / before)
    return "same" if pct == 0 else f"{pct:+d}%".replace("-", "−")


def main():
    results = json.loads((HERE / "results.json").read_text())
    kinds = {p["name"]: p["kind"] for p in tomllib.loads((HERE / "corpus.toml").read_text())["project"]}
    rows = [r for r in results if r["flags"] == "default" and r["project"] != "edge-cases"]
    rows.sort(key=lambda r: r["stdout_lines"])

    lines = [
        "| Project | What it is | vulture | condor | Change |",
        "|---|---|---:|---:|---:|",
    ]
    for r in rows:
        kind = kinds.get(r["project"], "").split(";")[0].split("(")[0].strip()
        lines.append(
            f"| {r['project']} | {kind} | {r['stdout_lines']:,} | {r['condor_lines']:,} | "
            f"{change(r['stdout_lines'], r['condor_lines'])} |"
        )
    v_total = sum(r["stdout_lines"] for r in rows)
    c_total = sum(r["condor_lines"] for r in rows)
    lines.append(f"| **Total** | | **{v_total:,}** | **{c_total:,}** | **{change(v_total, c_total)}** |")
    same_exit = all(r["exit_equal"] for r in results)
    lines += [
        "",
        f"Findings reported with default options. Exit codes: "
        f"{'identical in all' if same_exit else 'different in some of the'} {len(results)} runs, "
        "including the other option sets.",
    ]
    (HERE / "SUMMARY.md").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
