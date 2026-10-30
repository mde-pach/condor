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

`harness/bench.py` writes `harness/benchmark.json`; `python harness/charts.py`
redraws the charts from it. `scripts/hero_svg.py FONT_DIR` redraws the
README animation (JetBrains Mono font files in `FONT_DIR`).

Update the bundled plugins with `scripts/sync-plugins.sh ../condor-plugins`.
Pushing a `v*` tag publishes `cndr` to PyPI.
