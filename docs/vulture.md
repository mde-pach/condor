# Coming from vulture

Replace `vulture` with `condor` (or `uvx cndr`). Options, `[tool.vulture]`,
`# noqa` comments, whitelists and exit codes keep working.

## Better

- **Much faster**: see the [results](results.md).
- **Fewer false positives**: framework code (pytest fixtures, pydantic and
  dataclass fields, enum members, Django, Flask, FastAPI, click, Celery),
  protocol and abstract method parameters, and entry points in
  `pyproject.toml` are recognised.
- **More dead code found**: a function or class that only refers to itself
  is reported.
- **Stale whitelist entries** are reported instead of silently hiding new
  dead code.
- **New options**: `--format json|sarif`, `--only KINDS`,
  `--no-report PATTERNS` (scan for uses without reporting), and directories
  are scanned without what `.gitignore` ignores.

## Different

- Syntax errors are worded differently (same exit status).
- `--help`, `--version` and `-v` output differ.
- A file with a malformed `# type:` comment is analysed instead of rejected.

The full specification is in the repository's
[`spec/`](https://github.com/mde-pach/condor/tree/main/spec) folder.
