# Development

## Layout

| Path | Contents |
|---|---|
| `crates/condor_core` | Language-agnostic engine: items, whole-project aggregation, report formats, glob patterns. |
| `crates/condor_python` | Python front end: parsing with ruff's parser, definitions and uses, reachability, noqa, bundled whitelists, plugin engine, and the embedded plugin snapshot (`plugins/`). |
| `crates/condor` | The `condor` command: options, configuration, file discovery, output. |
| `crates/condor_testapi` | Test-only bridge that lets vulture's own test suite drive condor. Not published. |
| `tests/upstream` | vulture 2.16's test suite (MIT) and its runner. |
| `harness` | Differential test against vulture on a pinned corpus, and the benchmark. |
| `docs` | This site (mkdocs). |

## Tests

```console
$ cargo test --workspace
$ cargo build && uv run --python 3.14 --with pytest python tests/upstream/run.py
$ cargo build --release && uv run --python 3.14 python harness/difftest.py
$ uv run --python 3.14 python harness/bench.py
```

The upstream runner runs vulture's tests unchanged, except for `call_vulture`,
which runs condor. Tests condor deliberately does not pass are marked as
expected failures with the reason (`tests/upstream/conftest_condor.py`).

## Plugins

Bundled plugins come from
[condor-plugins](https://github.com/mde-pach/condor-plugins). To update the
snapshot:

```console
$ scripts/sync-plugins.sh ../condor-plugins
```

## Documentation

```console
$ uvx --with mkdocs-material==9.7.6 mkdocs serve
```

The site is built and published to GitHub Pages on every push to `main`.
