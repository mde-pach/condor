# Differences from vulture 2.16

condor reports the same dead code with the same exit codes
([compatibility.md](compatibility.md)). It differs from vulture in the
following places. Each one is either outside condor's job (reproducing CPython
or argparse rather than finding dead code) or a case where vulture crashes.

| | Difference | Effect on findings or exit code |
|---|---|---|
| D1 | Syntax errors are worded with ruff's messages (`path:line: message`), not CPython's. | None: the file is rejected and the exit status is 1, as in vulture. |
| D2 | `usage`, `--help`, `--version` and argparse error messages are condor's own. `-v` does not print vulture's per-node `ast.dump` lines, and whitelists are included in alphabetical order (vulture: unspecified). | None. |
| D3 | Type comments are not checked against CPython's grammar. vulture rejects a whole file that has a misplaced `# type:` comment (for example `f()  # type: int`), and stops visiting a file part-way when the text of a type comment does not parse (`# type: *Any`); condor analyses these files fully. | condor reports the dead code of these files, and the names they use count as used, which can hide a finding vulture reports elsewhere. Seen in the corpus: CPython's `test_tarfile.py` is rejected by vulture, so vulture reports 5 `tarfile.py` methods that condor does not; in black's test data condor reports 3 more variables. The upstream test `test_invalid_type_comment` is marked as an expected failure. Kept deliberately (owner decision, 2026-10-29): analysing these files gives better dead-code detection. |
| D4 | Directories are walked in sorted order (vulture: file-system order). | None on findings, which are sorted; only the order of error lines on stderr. |
| D5 | Where vulture crashes with a traceback, condor prints one error line: `--min-confidence` outside 0–100, a file that cannot be read (for example a directory named `x.py`), an invalid TOML config, a working directory that no longer exists. A directory named `x.py` is skipped when walking a directory (vulture crashes) and scanned as a directory when given explicitly. An unhashable literal in a condition (`if {[]: 1}:`) is treated as non-constant instead of crashing. | Same exit status (1) except the last case, where vulture crashes and condor continues. |
| D6 | Source encodings: condor decodes UTF-8, Latin-1, ASCII, CP1252 and the common ISO-8859, Windows, Japanese, Chinese and Korean codecs. Python knows more. | A file in another declared encoding is reported as unreadable (exit 1) where vulture would analyse it. |
| D7 | `pyproject.toml` is read as TOML 1.1 (Python 3.14 reads TOML 1.0). | None for TOML 1.0 files. |
| D8 | `server`, `daemon` and `query` as the first argument are condor's commands, not paths. | To scan a directory with one of these names, write `./server`. |
