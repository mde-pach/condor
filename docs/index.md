# condor

condor (PyPI: `cndr`) finds unused Python code: functions nobody calls, classes nobody uses,
imports and variables left behind, and code that can never run. It is a Rust
reimplementation of [vulture](https://github.com/jendrikseipp/vulture) 2.16
that keeps vulture's options, configuration and exit codes, runs about 20
times faster, and reports fewer false positives.

```console
$ condor src/
src/app/loader.py:12: unused function 'useful_load' (60% confidence)
src/app/models.py:3: unused import 'json' (90% confidence)
src/app/worker.py:41: unreachable code after 'return' (100% confidence)
```

## Install

condor is published on PyPI as `cndr`, with prebuilt binaries for Linux,
macOS and Windows. It installs two identical commands, `condor` and `cndr`.

```console
$ uvx cndr .                      # run once, without installing
$ uv tool install cndr            # install the `condor` command
$ uv add --dev cndr               # add it to a project's dev dependencies
$ pip install cndr
```

Building from source (for platforms without a prebuilt binary, or with
`cargo install --git https://github.com/mde-pach/condor condor`) needs Rust
1.97 or later.

## Use

```console
$ condor PATH [PATH ...]             # files and directories
$ condor . --min-confidence 80       # only the most certain findings
$ condor . --format json             # or sarif, for CI and code scanning
$ condor . --make-whitelist > whitelist.py
```

Directories are scanned for `*.py` files, without what the git repository's
`.gitignore` ignores. Exit status: 0 no dead code, 1 invalid input (a file
that cannot be parsed), 2 invalid options, 3 dead code found.

| Option | Effect |
|---|---|
| `--exclude PATTERNS` | skip files matching these comma-separated globs |
| `--no-report PATTERNS` | scan these files for uses, but do not report their findings |
| `--ignore-names PATTERNS` | never report these names |
| `--ignore-decorators PATTERNS` | never report definitions with these decorators |
| `--only KINDS` | report only these kinds (`function,class,...` or `V103,...`) |
| `--min-confidence N` | report findings with at least N% confidence |
| `--sort-by-size` | largest unused code last, with its size |
| `--make-whitelist` | print findings as a whitelist file |
| `--format text\|json\|sarif` | output format |
| `--config FILE` | configuration file (default `pyproject.toml`) |

## Configure

condor reads `[tool.vulture]` and `[tool.condor]` in `pyproject.toml`
(`[tool.condor]` wins). Keys are the option names with underscores, plus
`paths` and `plugins`:

```toml
[tool.condor]
paths = ["src", "tests"]
exclude = ["*/migrations/*"]
min_confidence = 60
plugins = ["condor-rules"]   # your own plugin files or directories
```

## False positives

Some code is used in ways a name-based analysis cannot see: a framework calls
it, a library reads it by name. condor knows the common cases through
[plugins](plugins.md) (pytest, pydantic, Django, Flask, FastAPI and others),
and you can add your own. For a one-off, add `# noqa` to the line, add the
name to `--ignore-names`, or keep a whitelist file made with
`--make-whitelist` and pass it as a path.

## Coming from vulture

Replace `vulture` with `condor` in your command or CI job; options and
`[tool.vulture]` keep working. condor reports less in most projects, because
of its [improvements](improvements.md), and occasionally more (code that only
refers to itself). The [vulture behaviour](compatibility.md) it starts from
and its [other differences](divergences.md) are documented, and the
[results](results.md) page compares both tools on real projects.
