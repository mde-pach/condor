# Improvements over vulture

condor starts from vulture 2.16's behaviour ([compatibility.md](compatibility.md))
and changes it where vulture is known to be wrong or limited. These rules come
from vulture's issue tracker. They are condor's default behaviour; there is no
option to turn them off.

Rules that depend on a library (parts of I1, I2 and I3) are
[plugins](../docs/plugins.md), which projects can extend; the others are built into
the engine.

## Fewer false positives

**I1. Interface parameters.** The parameters of these functions are not
reported, because the signature is imposed by something else:

* functions whose body is only a docstring, `...` or
  `raise NotImplementedError` (built in);
* methods of `Protocol` classes, and functions decorated with
  `@abstractmethod`, `@overload` or `@override` (plugins `typing`, `abc`).

A `pass` body does not count, since it is as often a placeholder as an
interface. The functions themselves are still reported when unused.
(vulture #309, #317, #410)

**Module `__getattr__` and `__dir__`.** Python calls them on attribute access
to the module (PEP 562), so they are used.

**I2. Declarative class fields.** Fields are set by construction and read by
libraries (serialisation, validation, iteration), so they count as used
(plugins `dataclasses`, `enum`, `typing`, `pydantic`, `attrs`, `msgspec`,
`sqlmodel`, `django`):

* annotated attributes of dataclasses, `TypedDict`, `NamedTuple`, pydantic
  models, msgspec structs and SQLModel models, except `ClassVar` ones;
  annotated attributes of attrs classes and those assigned `attr.ib()`;
* members of `Enum`, `IntEnum`, `StrEnum`, `Flag` and `IntFlag` classes;
* Django model and form fields (attributes assigned a call, such as
  `models.CharField()`), `Meta` options, and the options Django defines for
  views, admin classes, commands, app configs and migrations.

Subclasses count too, across files. Trade-off: a field or enum member that is
genuinely never read is no longer reported. (vulture #362, #335, #304)

**I3. pytest fixtures** (plugin `pytest`). A fixture (`@pytest.fixture`,
`@pytest_asyncio.fixture`) is used when a test or another fixture takes a
parameter with its name (or the name given by `name=`), or when
`@pytest.mark.usefixtures` names it. `autouse=True` fixtures are always used.
The parameters of tests and fixtures are not reported, since pytest passes
them. (vulture #382)

**I4. Entry points.** Objects named in `[project.scripts]`,
`[project.gui-scripts]`, `[project.entry-points.*]` and
`[tool.poetry.scripts]` of the configuration file are used
(`tool = "pkg.cli:main"` uses `main`). (vulture #388)

**I5. Generator stubs.** A bare `yield` after `raise NotImplementedError`,
which makes an abstract method a generator, is not reported as unreachable.
(vulture #298)

## Dead code vulture misses

**I6. Self-references.** A function that only calls itself, a method only
called as `self.m()`/`cls.m()` from its own body, and a class only referred to
from its own body are unused. `super().m()` and delegation to another object
(`self.inner.m()`) still count as uses. A function that binds its own name
locally (`def data(): data = ...`) is not affected. (vulture #354, #407)

**I7. Stale whitelist entries.** In a file made with `--make-whitelist`, an
entry such as `name  # unused function (pkg/a.py:4)` is reported when
`pkg/a.py` was scanned but no longer defines `name`, as
`stale whitelist entry 'name': pkg/a.py no longer defines it` (V301, 100%
confidence). Its use no longer counts, so it cannot hide new dead code with
the same name elsewhere. A name the file still defines is not stale, even
when it is no longer reported (`# noqa`, `--ignore-names`, framework fields).
Entry paths are resolved against the working directory or the whitelist's
directory. (vulture #430)

**I4 detail.** An entry point `pkg.cli:main` uses `main` in `pkg/cli.py`;
only when that file is not scanned does it use every `main`.

**Framework entry points** (plugins `unittest`, `django`, `celery`, `flask`, `fastapi`,
`click`). Decorator patterns such as `*.get` and class names such as `Meta`
apply in files that import the framework, or to decorators whose object is
imported (`from myapp import app` then `@app.route`). Django model, view,
form (`clean_<field>`), command and app hooks, `Meta` classes, admin classes
registered with `@admin.register`, app configs, migrations, template filters
and tags, and signal receivers; Celery tasks; Flask views, hooks, error
handlers and template helpers; FastAPI path operations, event handlers,
middleware and exception handlers; click commands and groups; test case
classes (`unittest.TestCase`, Django's test cases), whatever their name, are
used. For
Django, the hooks are the public methods of Django's base classes, so an
override is used; its parameters, and those of signal receivers, are not
reported, since Django imposes them.

Classes are told apart by where they are defined, not by name: a base class
resolves through the file's imports, through re-exports (`from .base import
Base` in a package), and within a file to the nearest class of that name
defined before. Only a name brought in by a star import can match any
project class of that name.

## Usability

**I8. Output formats.** `--format json` prints one object per finding (path,
line, end line, kind, code, name, message, confidence, size). `--format sarif`
prints a SARIF 2.1.0 log for code-scanning tools: paths are URIs relative to
`%SRCROOT%` (the working directory), and files that could not be analysed are
listed as tool execution notifications. In both formats, log lines (`-v`) go
to stderr, so stdout is only the report; `--make-whitelist` cannot be
combined with them. (vulture #405)

**I9. `.gitignore`.** When a directory is scanned, paths ignored by
`.gitignore`, `.git/info/exclude` and `.ignore` files are skipped (virtual
environments, build output). Files given explicitly are always scanned.
(vulture #344)

**I10. Scan without reporting.** `--no-report PATTERNS` (`no_report` in the
configuration) scans the matching files, so the names they use count, but does
not report their own findings. `--exclude` still drops files entirely.
Patterns follow `--exclude`: one without wildcards matches anywhere in the
path (`tests`), one with wildcards must match the whole absolute path
(`*/tests/*`, not `tests/*`). (vulture #434)

**I11. Kinds.** `--only KINDS` (`only` in the configuration) reports only the
given kinds: `attribute`, `class`, `function`, `import`, `method`, `property`,
`variable`, `unreachable_code`, `whitelist_entry`, or their codes (`V103`),
in any case. (vulture #303)

## Configuration

condor reads `[tool.vulture]`, then `[tool.condor]`, whose keys take
precedence. The new keys (`no_report`, `only`, `format`, `plugins`) are
accepted in both; put them in `[tool.condor]` if vulture also reads the file,
since vulture rejects unknown keys. `plugins` paths are relative to the
configuration file. `--config` naming a missing file is an error (vulture
ignores it).
