# condor

condor finds unused Python code. It is a Rust reimplementation of
[vulture](https://github.com/jendrikseipp/vulture) 2.16 (same options,
`[tool.vulture]` configuration and exit codes), about 20 times faster, with
fewer false positives and a few more true ones.

```
condor src/ --min-confidence 80
condor . --format json
```

Documentation: https://mde-pach.github.io/condor/

* [docs/compatibility.md](docs/compatibility.md): the vulture behaviour condor starts from.
* [docs/improvements.md](docs/improvements.md): where condor does better, and how.
* [docs/plugins.md](docs/plugins.md): rules for libraries, and how to add your own.
* [docs/divergences.md](docs/divergences.md): other differences.
* [harness/REPORT.md](harness/REPORT.md): vulture vs condor on real projects.
* [harness/BENCHMARK.md](harness/BENCHMARK.md): timings.

## Layout

| Path | Contents |
|---|---|
| `crates/condor_core` | Language-agnostic engine: items, whole-project aggregation, report format, glob patterns. |
| `crates/condor_python` | Python front end: parsing (ruff's parser), definitions and uses, reachability, noqa, bundled whitelists. |
| `crates/condor` | The `condor` command: options, configuration, file discovery. |
| `crates/condor_testapi` | Test-only bridge that lets vulture's own test suite drive condor. Not published. |
| `tests/upstream` | vulture 2.16's test suite (MIT) and the runner. |
| `harness` | Differential test against vulture on a pinned corpus, and the benchmark. |

## Install

```
uvx cndr .              # run without installing
uv tool install cndr    # or: pip install cndr
```

The PyPI package is `cndr`; it installs the commands `condor` and `cndr`.
Building from source needs Rust 1.97 or later.

## Testing

```
cargo test --workspace
cargo build && uv run --python 3.14 --with pytest python tests/upstream/run.py
cargo build --release && uv run --python 3.14 python harness/difftest.py
uv run --python 3.14 python harness/bench.py
```

## Licence

MIT. vulture's bundled whitelists (`crates/condor_python/src/whitelists`) and
test suite (`tests/upstream/vulture-2.16`) are copyright Jendrik Seipp, MIT.
