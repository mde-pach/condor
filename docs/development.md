# Development

| Path | Contents |
|---|---|
| `crates/condor_core` | language-agnostic engine |
| `crates/condor_python` | Python analysis, plugin engine, bundled plugins |
| `crates/condor` | the command line |
| `tests/upstream` | vulture's own test suite, run against condor |
| `harness` | comparison with vulture on real projects, benchmark |
| `spec` | detailed specification |

```console
$ cargo test --workspace
$ uv run --python 3.14 --with pytest python tests/upstream/run.py
$ uv run --python 3.14 python harness/difftest.py
$ uvx --with mkdocs-material==9.7.6 mkdocs serve
```

What the tests check:

* `cargo test`: rules on small examples; the command line, `query`, the
  daemon and the language server; random edit sequences that must match a
  full scan (`CONDOR_TEST_SEEDS=200` for a longer run); and each bundled
  plugin's cases (condor-plugins `tests/`), where every line marked
  `# dead: name` must be reported and nothing else may be.
* `harness/difftest.py`: findings and exit codes against vulture on real
  projects.
* `harness/plant.py`: plants uniquely named dead code in every file of those
  projects (functions, classes, methods, attributes, parameters, imports,
  self-references) and lists each plant condor does not report with the
  rule that drops it (`harness/PLANT.md`).
* `harness/drops.py`: attributes each finding only vulture reports to the
  plugin rule or built-in rule that drops it, and samples the findings only
  condor reports, for review (`harness/DROPS.md`).

`harness/bench.py` writes `harness/benchmark.json`; `python harness/charts.py`
redraws the charts from it. `scripts/hero_svg.py FONT_DIR` redraws the
README animation (JetBrains Mono font files in `FONT_DIR`).

Update the bundled plugins with `scripts/sync-plugins.sh ../condor-plugins`.
Pushing a `v*` tag publishes `cndr` to PyPI.
