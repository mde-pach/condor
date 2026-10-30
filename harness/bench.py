"""Benchmark vulture 2.16 against condor on the harness corpus.

Usage (after `cargo build --release` and a harness run, which clones the
corpus into target/corpus):

    uv run --python 3.14 python harness/bench.py

vulture runs from its own virtual environment (target/bench-venv, CPython
3.14) so that `uvx` start-up time is not counted. Timings are hyperfine's,
with warm-up runs, so the file system cache is warm. Results go to
harness/BENCHMARK.md.
"""

import json
import os
import platform
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
CORPUS = ROOT / "target" / "corpus"
VENV = ROOT / "target" / "bench-venv"
CONDOR = ROOT / "target" / "release" / "condor"

# (label, working directory, paths)
CASES = [
    ("one small file", CORPUS / "edge-cases", ["syntax/async_code.py"]),
    ("requests", CORPUS / "requests", ["."]),
    ("flask", CORPUS / "flask", ["."]),
    ("rich", CORPUS / "rich", ["."]),
    ("fastapi", CORPUS / "fastapi", ["."]),
    ("django", CORPUS / "django", ["."]),
    ("saleor", CORPUS / "saleor", ["."]),
]


def count_files(cwd, paths):
    n = 0
    for p in paths:
        path = cwd / p
        n += 1 if path.is_file() else sum(1 for _ in path.rglob("*.py"))
    return n


def main():
    if not (VENV / "bin" / "vulture").exists():
        subprocess.run(["uv", "venv", "-q", "--python", "3.14", str(VENV)], check=True)
        subprocess.run(["uv", "pip", "install", "-q", "--python", str(VENV / "bin" / "python"), "vulture==2.16"], check=True)
    vulture = VENV / "bin" / "vulture"
    rows = []
    for label, cwd, paths in CASES:
        if not cwd.exists():
            print(f"skip {label}: {cwd} missing (run harness/difftest.py first)")
            continue
        out = ROOT / "target" / f"bench-{label.replace(' ', '-')}.json"
        args = " ".join(paths)
        runs = "20" if label == "one small file" else "5"
        subprocess.run(
            ["hyperfine", "-N", "-i", "--warmup", "2", "--runs", runs, "--export-json", str(out),
             "-n", "vulture", f"{vulture} {args}", "-n", "condor", f"{CONDOR} {args}"],
            cwd=cwd, check=True, stdout=subprocess.DEVNULL,
        )
        results = {r["command"]: r for r in json.loads(out.read_text())["results"]}
        v, c = results["vulture"], results["condor"]
        rows.append((label, count_files(cwd, paths), v["mean"], v["stddev"], c["mean"], c["stddev"], v["mean"] / c["mean"]))
        print(f"{label:16} files={rows[-1][1]:6} vulture={v['mean']:.3f}s condor={c['mean']:.3f}s x{rows[-1][6]:.1f}")

    cpu = subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip()
    load = os.getloadavg()[0]
    lines = [
        "# Benchmark",
        "",
        f"Machine: {cpu or platform.processor()}, {os.cpu_count()} cores, {platform.system()}. "
        f"vulture 2.16 on CPython {sys.version.split()[0]}; condor release build.",
        "Mean wall-clock time over several runs after two warm-up runs (hyperfine), so",
        "the file system cache is warm. Same corpus revisions as REPORT.md.",
        f"Load average when the run ended: {load:.1f} on {os.cpu_count()} cores"
        + (" (busy machine: timings are slower than usual for both tools)." if load > os.cpu_count() / 2 else "."),
        "",
        "| Input | .py files | vulture (s) | condor (s) | Speed-up |",
        "|---|---:|---:|---:|---:|",
    ]
    for label, n, vm, vs, cm, cs, ratio in rows:
        lines.append(f"| {label} | {n} | {vm:.3f} ± {vs:.3f} | {cm:.3f} ± {cs:.3f} | {ratio:.0f}× |")
    (HERE / "BENCHMARK.md").write_text("\n".join(lines) + "\n")
    data = {
        "machine": cpu or platform.processor(),
        "cores": os.cpu_count(),
        "python": sys.version.split()[0],
        "load_average": round(load, 1),
        "runs": [
            {"input": label, "files": n, "vulture_s": round(vm, 4), "condor_s": round(cm, 4)}
            for label, n, vm, vs, cm, cs, ratio in rows
        ],
    }
    (HERE / "benchmark.json").write_text(json.dumps(data, indent=1) + "\n")


if __name__ == "__main__":
    main()
