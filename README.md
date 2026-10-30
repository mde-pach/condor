<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/mde-pach/condor/main/docs/assets/hero-dark.svg">
  <img alt="condor: its two unused letters o are flagged and removed, leaving cndr" src="https://raw.githubusercontent.com/mde-pach/condor/main/docs/assets/hero-light.svg" width="600">
</picture>

Finds unused Python code: functions nobody calls, classes nobody uses,
leftover imports and variables, unreachable code. A faster
[vulture](https://github.com/jendrikseipp/vulture) with fewer false positives.

```
uvx cndr src/
```

Install with `uv tool install cndr` (or `pip install cndr`); the commands are
`condor` and `cndr`. Wheels cover Linux, macOS and Windows on x86-64 and
ARM; elsewhere pip builds from source, which needs Rust 1.97 or later.

**Documentation: https://mde-pach.github.io/condor/**

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/mde-pach/condor/main/docs/assets/bench-dark.svg">
  <img alt="Scanning Django: condor versus vulture" src="https://raw.githubusercontent.com/mde-pach/condor/main/docs/assets/bench-light.svg" width="720">
</picture>

More in the [results](https://mde-pach.github.io/condor/results/).

## Repository

| Path | Contents |
|---|---|
| `crates/` | condor in Rust: engine, Python analysis, command line |
| `tests/upstream/` | vulture's own test suite, run against condor |
| `harness/` | comparison with vulture on real projects ([summary](https://github.com/mde-pach/condor/blob/main/harness/SUMMARY.md)), benchmark |
| `spec/` | detailed specification: [vulture behaviour](https://github.com/mde-pach/condor/blob/main/spec/compatibility.md), [improvements](https://github.com/mde-pach/condor/blob/main/spec/improvements.md), [differences](https://github.com/mde-pach/condor/blob/main/spec/divergences.md) |
| `docs/` | the documentation site |

## Licence

MIT. condor includes vulture's whitelists and test suite, copyright Jendrik
Seipp, MIT ([NOTICE](https://github.com/mde-pach/condor/blob/main/NOTICE)).
