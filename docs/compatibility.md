# vulture 2.16 compatibility

This is vulture 2.16's behaviour, which condor reproduces: options,
configuration, confidence values, order, output format and exit codes. condor
then deliberately improves on it; those rules are in
[improvements.md](improvements.md). Other differences (the wording of syntax
errors, `--help`, `-v` logs) are listed in [divergences.md](divergences.md).

Source references are to vulture 2.16 (`vulture/<file>.py:<line>`, tag
`v2.16`). vulture is MIT-licensed, copyright Jendrik Seipp. The reference
interpreter is CPython 3.14: vulture accepts whatever syntax the interpreter
running it accepts.

## Options and configuration (`config.py`)

* Options: `PATH ...`, `--exclude`, `--ignore-decorators`, `--ignore-names`
  (comma-separated glob patterns), `--make-whitelist`, `--min-confidence N`,
  `--sort-by-size`, `--config FILE` (default `pyproject.toml`), `-v`,
  `--version`, `-h`. Long options can be abbreviated to any unambiguous prefix
  and written `--opt=value`. An invalid command line exits with status 2.
* `[tool.vulture]` in the config file, with the keys `paths`, `exclude`,
  `ignore_decorators`, `ignore_names`, `make_whitelist`, `min_confidence`,
  `sort_by_size` and `verbose`. An unknown key or a value of the wrong type
  exits with status 2. The command line overrides the file. No paths at all
  exits with status 2.

## Files (`utils.py:82-111`, `core.py:267-313`)

* A file given on the command line is scanned whatever its extension; a
  `.pyc` file or a missing path exits with status 1. A directory contributes
  its `*.py` files, recursively (not `*.pyi`).
* `--exclude`: a pattern without `*`, `?` or `[` becomes `*pattern*`; patterns
  are matched against the absolute path.
* Files are decoded like Python source (BOM, coding cookie). A file that cannot
  be decoded or parsed contributes nothing and sets exit status 1.

## What is defined (`core.py:471-661`)

| Construct | Reported as | Confidence |
|---|---|---|
| `import x`, `from m import x` (alias if given; first component of dotted names; not `__future__`) | import | 90 |
| `class C` | class | 60 |
| `def f` | function | 60 |
| `def m` with `@staticmethod`, `@classmethod`, or first parameter `self` | method | 60 |
| `def p` with `@property` | property | 60 |
| `x.attr = ...` | attribute | 60 |
| `x = ...` and other name bindings | variable | 60 |
| function and lambda parameters | variable | 100 |
| unreachable code (§ Reachability) | unreachable code | 100 |

A definition starts at its first decorator and ends at its last line.

**Not reported:**

* Imports in `__init__.py` files, and `*` imports.
* Variables named `_x` (but not `__x`), dunder names, `object` and `self`.
* Dunder methods.
* In test files (path matches `*/test/*`, `*/tests/*`, `*/test*.py` or
  `*[-_]test.py`): `test_*` functions and methods, pytest's
  `setup_module`/`teardown_module`/`setup_function`/`teardown_function`
  functions and `setup_class`/`teardown_class`/`setup_method`/
  `teardown_method` methods, and classes whose name contains `Test`.
* Names matching `--ignore-names`.
* Classes and functions with a decorator matching `--ignore-decorators`.
  Decorators are named `@` plus their dotted name (`@app.route` for
  `@app.route("/")`).
* Lines with `# noqa` (all codes) or `# noqa: <codes>`: V101 attribute, V102
  class, V103 function, V104 import, V105 method, V106 property, V107
  variable, V201 unreachable code; F401 means V104 and F841 means V107.

## What counts as used

A definition is unused when its name is used nowhere in the project. Uses:

* names read or deleted, and attribute names read (`x.name`);
* the strings in `__all__ = [...]` or `(...)`;
* `getattr(x, "name")`, `getattr(x, "name", default)`, `hasattr(x, "name")`;
* `"%(name)s" % locals()` and `"{name}".format(**locals())`;
* keyword attributes of `match` class patterns (`case P(x=...)`);
* names in `# type:` comments;
* the full dotted name of an aliased import.

## Bundled whitelists (`core.py:299-313`)

When a module named argparse, ast, collections, ctypes, enum, logging, pint,
socketserver, ssl, string, sys, threading or unittest is imported, vulture's
whitelist for it is scanned as well (unless `--exclude` matches
`whitelists/<name>_whitelist.py`). vulture looks these files up on disk, so on
case-insensitive file systems (macOS, Windows) `from enum import Enum` also
brings in the enum whitelist; condor matches case-insensitively on those
platforms and case-sensitively elsewhere. Users' own whitelists are ordinary
files passed as paths.

## Reachability (`reachability.py`, `utils.py:19-53`)

* After `return`, `raise`, `break` or `continue`, the rest of the block is
  `unreachable code after '<statement>'`. A statement also ends its block when
  all of its branches do: an `if` whose branches all end, a `try` whose body
  and handlers all end, or a `while True` loop without `break`. Bodies of
  modules, functions, `with` and `for` are checked; class bodies, `else` and
  `finally` blocks, `match` cases and `try*` are not.
* Constant conditions are evaluated with `ast.literal_eval` truthiness:
  `unsatisfiable 'if' condition`, `unreachable 'else' block`,
  `redundant if-condition`, `unsatisfiable 'while' condition`,
  `unsatisfiable 'ternary' condition` and `unreachable 'else' expression`.

## Output (`core.py:116-188, 315-365`)

* `<path>:<line>: <message> (<confidence>% confidence)`, with
  `, <n> line(s)` before the `)` when `--sort-by-size` is given. Paths are
  shown relative to the current directory when they are inside it.
* `--make-whitelist`: `<name>  # unused <type> (<path>:<line>)`, `_.<name>`
  for attributes, methods and properties, and `# <message> (<path>:<line>)`
  for unreachable code.
* Order: by path (lowercase) then line; with `--sort-by-size`, by size first.
  Items on the same line come in the order attribute, class, function, import,
  method, property, variable, unreachable code.
* `--min-confidence N` keeps findings with confidence ≥ N.
* Exit status: 0 no dead code, 1 invalid input, 2 invalid command line,
  3 dead code found (3 wins over 1).
