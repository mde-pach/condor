"""Plant dead code in the corpus projects and check that condor reports it.

For each project in corpus.toml, a copy-on-write clone gets uniquely named
dead code in every file it tracks: module-level functions, classes,
variables and imports, a function that only calls itself, and in every class
a method, a method that only calls itself and an attribute, plus an unused
parameter on every function. condor runs with default options and must
report every plant, except where a rule documents the drop (a class
attribute that a framework treats as a field, a parameter whose signature is
imposed). Every plant condor does not report is listed with its context, for
review. vulture runs too, as a reference.

Usage (from the repository root, after `cargo build --release` and
`python harness/difftest.py`, which clones the corpus):

    uv run --python 3.14 python harness/plant.py [--only NAME ...] [--jobs N]

Writes harness/PLANT.md and target/plant/results.json.
"""

import argparse
import ast
import collections
import concurrent.futures
import json
import re
import shutil
import subprocess
import sys
import sysconfig
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
CACHE = ROOT / "target" / "corpus"
WORK = ROOT / "target" / "plant"
CONDOR = ROOT / "target" / "release" / "condor"
TESTAPI = ROOT / "target" / "release" / "condor_testapi"
VULTURE = ["uvx", "-q", "--python", "3.14", "--from", "vulture==2.16", "vulture"]
PLANT = re.compile(r"'((?:cndr_plant|CndrPlant)_[a-z_]+_\d+)'")

# Kinds of plants, and what condor must do with them.
KINDS = {
    "function": "module-level function",
    "recursive": "function that only calls itself (vulture misses it)",
    "class": "module-level class",
    "variable": "module-level variable",
    "import": "unused import",
    "method": "method",
    "self_method": "method that only calls itself (vulture misses it)",
    "attribute": "class attribute (a framework may treat it as a field)",
    "parameter": "parameter (a signature may be imposed)",
}


def source_root(project):
    if project.get("local") == "python3.14-stdlib":
        return Path(sysconfig.get_paths()["stdlib"])
    if "local" in project:
        return None
    return CACHE / project["name"]


def tracked_files(root, paths):
    """Python files in `paths` that git tracks (all of them outside git)."""
    files = []
    for p in paths:
        target = root / p
        if target.is_file():
            files.append(target)
            continue
        listed = subprocess.run(["git", "ls-files", "-z", "--", p], cwd=root, capture_output=True)
        if listed.returncode == 0 and listed.stdout:
            files += [root / f for f in listed.stdout.decode().split("\0") if f.endswith(".py")]
        else:
            files += [f for f in target.rglob("*.py") if "site-packages" not in f.parts]
    return sorted(set(files))


class Planter:
    def __init__(self):
        self.count = 0
        self.plants = {}  # name -> context

    def name(self, kind, context):
        self.count += 1
        prefix = "CndrPlant" if kind == "class" else "cndr_plant"
        name = f"{prefix}_{kind}_{self.count}"
        self.plants[name] = {"kind": kind, **context}
        return name

    def plant(self, path, rel):
        try:
            text = path.read_bytes().decode("utf-8")
        except (UnicodeDecodeError, OSError):
            return
        if "\r" in text or "\f" in text:
            return
        try:
            tree = ast.parse(text)
        except (SyntaxError, ValueError):
            return
        lines = text.splitlines(keepends=True)
        if lines and not lines[-1].endswith("\n"):
            lines[-1] += "\n"
        before = len(self.plants)
        # (line index to insert before, [lines]); header rewrites by line span.
        inserts = []
        rewrites = []
        parents = {}
        for node in ast.walk(tree):
            for child in ast.iter_child_nodes(node):
                parents[child] = node

        def enclosing_class(node):
            p = parents.get(node)
            return p if isinstance(p, ast.ClassDef) else None

        def describe_class(c):
            return {
                "class": c.name,
                "bases": [ast.unparse(b) for b in c.bases] + [ast.unparse(k) for k in c.keywords],
                "class_decorators": [ast.unparse(d) for d in c.decorator_list],
            }

        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef):
                first = node.body[0]
                line = lines[first.lineno - 1]
                if line[: byte_col(line, first.col_offset)].strip():
                    continue  # the body starts on a header line: `class A: pass`
                indent = re.match(r"[ \t]*", lines[first.lineno - 1]).group()
                ctx = {"file": rel, **describe_class(node)}
                m = self.name("method", ctx)
                s = self.name("self_method", ctx)
                a = self.name("attribute", ctx)
                inserts.append((node.end_lineno, [
                    f"{indent}{a} = 1\n",
                    f"{indent}def {m}(self):\n", f"{indent}    return 1\n",
                    f"{indent}def {s}(self):\n", f"{indent}    return self.{s}()\n",
                ]))
            elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                body = node.body[0]
                start = (node.lineno, node.col_offset)
                end = (body.lineno, body.col_offset)
                header = slice_text(lines, start, end)
                colon = header.rstrip().rfind(":")
                if "#" in header or colon < 0 or "\\" in header:
                    continue
                cls = enclosing_class(node)
                ctx = {
                    "file": rel,
                    "function": node.name,
                    "decorators": [ast.unparse(d) for d in node.decorator_list],
                    "stub": is_stub(node),
                    **(describe_class(cls) if cls else {}),
                }
                p = self.name("parameter", ctx)
                copy = ast.FunctionDef(
                    name=node.name, args=node.args, body=[ast.Pass()], decorator_list=[],
                    returns=node.returns, type_params=getattr(node, "type_params", []),
                )
                new_args = ast.arguments(
                    posonlyargs=node.args.posonlyargs, args=node.args.args, vararg=node.args.vararg,
                    kwonlyargs=[*node.args.kwonlyargs, ast.arg(arg=p)],
                    kw_defaults=[*node.args.kw_defaults, ast.Constant(None)],
                    kwarg=node.args.kwarg, defaults=node.args.defaults,
                )
                copy.args = new_args
                copy.lineno, copy.type_comment = node.lineno, None
                new_header = ast.unparse(copy).split("\n")[0]
                if isinstance(node, ast.AsyncFunctionDef):
                    new_header = "async " + new_header
                # Keep everything after the header's colon (newline, indent).
                rewrites.append((start, start_plus(header, start, colon + 1), new_header))

        rel_ctx = {"file": rel}
        f = self.name("function", rel_ctx)
        r = self.name("recursive", rel_ctx)
        c = self.name("class", rel_ctx)
        v = self.name("variable", rel_ctx)
        i = self.name("import", rel_ctx)
        tail = [
            "\n\n",
            f"def {f}():\n", "    return 1\n", "\n\n",
            f"def {r}(n):\n", f"    return {r}(n)\n", "\n\n",
            f"class {c}:\n", "    pass\n", "\n\n",
            f"{v} = 1\n",
            f"import {i}\n",
        ]
        # Apply header rewrites (same-line edits from the right), then
        # inserts from the bottom so line numbers stay valid.
        for (sl, sc), (el, ec), new in sorted(rewrites, reverse=True):
            prefix = lines[sl - 1][: byte_col(lines[sl - 1], sc)]
            suffix = lines[el - 1][byte_col(lines[el - 1], ec):]
            lines[sl - 1: el] = [prefix + new + suffix]
            # Later (higher) spans were already applied; earlier ones are
            # above sl, unaffected. Inserts below shift by the removed lines.
            shift = el - sl
            inserts = [(at - shift if at >= el else at, block) for at, block in inserts]
        for at, block in sorted(inserts, key=lambda x: x[0], reverse=True):
            lines[at:at] = block
        lines += tail
        new_text = "".join(lines)
        try:
            ast.parse(new_text)
        except SyntaxError:
            for name in list(self.plants)[before:]:
                del self.plants[name]
            return
        path.write_text(new_text, encoding="utf-8")


def byte_col(line, col):
    """ast columns are UTF-8 byte offsets; return the str index."""
    return len(line.encode("utf-8")[:col].decode("utf-8", "ignore"))


def slice_text(lines, start, end):
    (sl, sc), (el, ec) = start, end
    if sl == el:
        line = lines[sl - 1]
        return line[byte_col(line, sc): byte_col(line, ec)]
    out = [lines[sl - 1][byte_col(lines[sl - 1], sc):]]
    out += lines[sl: el - 1]
    out.append(lines[el - 1][: byte_col(lines[el - 1], ec)])
    return "".join(out)


def start_plus(text, start, n):
    """The (line, byte column) `n` characters into `text`, which starts at `start`."""
    line, col = start
    before = text[:n]
    newlines = before.count("\n")
    if newlines == 0:
        return (line, col + len(before.encode("utf-8")))
    return (line + newlines, len(before.rsplit("\n", 1)[1].encode("utf-8")))


def is_stub(node):
    body = node.body
    while body and isinstance(body[0], ast.Expr) and isinstance(body[0].value, ast.Constant) and isinstance(body[0].value.value, str):
        body = body[1:]
    if not body:
        return True
    if len(body) != 1:
        return False
    s = body[0]
    if isinstance(s, ast.Expr) and isinstance(s.value, ast.Constant) and s.value.value is Ellipsis:
        return True
    if isinstance(s, ast.Raise) and s.exc is not None:
        e = s.exc.func if isinstance(s.exc, ast.Call) else s.exc
        return isinstance(e, ast.Name) and e.id == "NotImplementedError"
    return False


def run_project(project):
    src = source_root(project)
    if src is None or not src.exists():
        return None
    src = src.resolve()
    dest = WORK / project["name"]
    if dest.exists():
        shutil.rmtree(dest)
    dest.parent.mkdir(parents=True, exist_ok=True)
    ignore = shutil.ignore_patterns("site-packages", "__pycache__")
    if sys.platform == "darwin":
        subprocess.run(["cp", "-Rc", str(src), str(dest)], check=True)
        shutil.rmtree(dest / "site-packages", ignore_errors=True)
    else:
        shutil.copytree(src, dest, ignore=ignore, symlinks=True)
    paths = project.get("paths", ["."])
    planter = Planter()
    for f in tracked_files(dest, paths):
        planter.plant(f, str(f.relative_to(dest)))
    flags = ["--exclude", "site-packages"] if project.get("local") == "python3.14-stdlib" else []
    reported = {}
    for tool, cmd in (("vulture", VULTURE), ("condor", [str(CONDOR)])):
        out = subprocess.run([*cmd, *flags, *paths], cwd=dest, capture_output=True).stdout.decode("utf-8", "replace")
        reported[tool] = set(PLANT.findall(out))
    rules = {}
    for removed in explain_drops(dest, [*flags, *paths]):
        if PLANT.fullmatch(f"'{removed['name']}'"):
            rules[removed["name"]] = removed["rule"]
    ignored = condor_ignored(dest, paths)
    for name, plant in planter.plants.items():
        if name in rules:
            plant["rule"] = rules[name]
        if plant["file"] in ignored:
            plant["ignored"] = True
    shutil.rmtree(dest)
    return {"project": project["name"], "plants": planter.plants, **{k: sorted(v) for k, v in reported.items()}}


def explain_drops(cwd, argv):
    """The items condor's plugins remove, with the rule (condor_testapi)."""
    request = json.dumps({"op": "explain", "cwd": str(cwd), "argv": argv})
    out = subprocess.run([str(TESTAPI)], input=request.encode(), capture_output=True, check=True).stdout
    return json.loads(out)


def condor_ignored(cwd, paths):
    """Files condor skips (.gitignore), relative to `cwd`."""
    out = subprocess.run([str(CONDOR), "-v", *paths], cwd=cwd, capture_output=True).stdout.decode("utf-8", "replace")
    scanned = {line.removeprefix("Scanning: ").strip() for line in out.splitlines() if line.startswith("Scanning: ")}
    scanned = {str(Path(p).resolve().relative_to(cwd.resolve())) if Path(p).is_absolute() else str(Path(p)) for p in scanned}
    return {str(f.relative_to(cwd)) for f in tracked_files(cwd, paths)} - scanned


def explain(plant):
    """Why a plant may legitimately be dropped, or None."""
    kind = plant["kind"]
    if "rule" in plant:
        return f"plugin {plant['rule']}"
    if plant.get("ignored"):
        return "file skipped by .gitignore (I9)"
    decorators = " ".join(plant.get("decorators", []))
    bases = " ".join(plant.get("bases", []))
    if kind == "import" and plant["file"].endswith("__init__.py"):
        return "import in __init__.py (vulture rule)"
    if kind == "parameter":
        if plant.get("stub"):
            return "stub body (I1)"
        if "Protocol" in bases:
            return "Protocol method (I1)"
        if re.search(r"abstract|overload|override", decorators):
            return "abstract/overload/override (I1)"
        if re.search(r"fixture", decorators):
            return "pytest fixture (I3)"
        if plant["function"].startswith("test") and re.search(r"(^|/)(test_[^/]*|[^/]*_test|conftest)\.py$", plant["file"]):
            return "pytest test (I3)"
    return None


def context_of(plant):
    if plant.get("decorators"):
        return "@" + ", @".join(plant["decorators"])[:60]
    if plant.get("class_decorators"):
        return "class @" + ", @".join(plant["class_decorators"])[:60]
    if plant.get("bases"):
        return "(" + ", ".join(plant["bases"])[:60] + ")"
    return plant.get("class") or plant["file"].rsplit("/", 1)[-1]


def summarize(results):
    lines = ["# Planted dead code", "",
             "Generated by `harness/plant.py`: uniquely named dead code planted in every",
             "tracked file of each corpus project, then condor and vulture run with",
             "default options. A plant condor does not report is a miss unless a",
             "documented rule drops it; the rest are listed for review.", ""]
    total = collections.Counter()
    found = collections.Counter()
    vfound = collections.Counter()
    vexpected = collections.Counter()
    unexplained = []
    explained = collections.Counter()
    examples = {}
    rows = []
    for r in results:
        cond, vult = set(r["condor"]), set(r["vulture"])
        per = collections.Counter()
        miss = 0
        for name, plant in r["plants"].items():
            kind = plant["kind"]
            total[kind] += 1
            if name in cond:
                found[kind] += 1
                continue
            if name in vult:
                vfound[kind] += 1
            why = explain(plant)
            if why:
                explained[(kind, why)] += 1
                examples.setdefault((kind, why), []).append((r["project"], plant))
            else:
                miss += 1
                per[kind] += 1
                unexplained.append((r["project"], name, plant, name in vult))
        for name in vult:
            vexpected[r["plants"][name]["kind"]] += 1
        rows.append((r["project"], len(r["plants"]), len(r["plants"]) - miss - sum(1 for n, p in r["plants"].items() if n not in cond and explain(p)), miss))
    lines += ["| Kind | Planted | condor reports | vulture reports |", "|---|---:|---:|---:|"]
    for kind, label in KINDS.items():
        lines.append(f"| {label} | {total[kind]:,} | {found[kind]:,} | {vexpected[kind]:,} |")
    lines += ["", "| Project | Planted | Unexplained misses |", "|---|---:|---:|"]
    for project, planted, _, miss in rows:
        lines.append(f"| {project} | {planted:,} | {miss:,} |")
    lines += ["", "## Dropped by a documented rule", "",
              "Each with the class bases (or decorators) seen most often, to review that the rule fits.", "",
              "| Kind | Reason | Plants | Most common contexts |", "|---|---|---:|---|"]
    for (kind, why), n in sorted(explained.items(), key=lambda x: -x[1]):
        contexts = collections.Counter(context_of(plant) for _, plant in examples[(kind, why)])
        shown = "; ".join(f"{c} ×{k}" for c, k in contexts.most_common(4))
        lines.append(f"| {kind} | {why} | {n:,} | {shown} |")
    lines += ["", f"## Not reported, without a documented reason ({len(unexplained):,})", ""]
    groups = collections.defaultdict(list)
    for project, name, plant, by_vulture in unexplained:
        key = (plant["kind"], tuple(plant.get("bases", [])), tuple(plant.get("class_decorators", [])), tuple(plant.get("decorators", [])))
        groups[key].append((project, name, plant, by_vulture))
    for key, items in sorted(groups.items(), key=lambda x: -len(x[1])):
        kind, bases, cdec, dec = key
        project, name, plant, by_vulture = items[0]
        lines.append(
            f"- {len(items)} × {kind}; bases {list(bases)}; class decorators {list(cdec)}; "
            f"decorators {list(dec)}; e.g. {project}/{plant['file']} {plant.get('class', '')}.{plant.get('function', '')} "
            f"({'vulture reports it' if by_vulture else 'vulture misses it too'})"
        )
    return "\n".join(lines) + "\n", unexplained


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--jobs", type=int, default=3)
    args = parser.parse_args()
    corpus = tomllib.loads((HERE / "corpus.toml").read_text())
    projects = [p for p in corpus["project"] if not args.only or p["name"] in args.only]
    results = []
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        for r in pool.map(run_project, projects):
            if r:
                print(f"{r['project']}: {len(r['plants'])} plants, condor {len(r['condor'])}, vulture {len(r['vulture'])}", flush=True)
                results.append(r)
    WORK.mkdir(parents=True, exist_ok=True)
    (WORK / "results.json").write_text(json.dumps(results))
    report, unexplained = summarize(results)
    (HERE / "PLANT.md").write_text(report)
    print(report)
    return 1 if unexplained else 0


if __name__ == "__main__":
    sys.exit(main())
