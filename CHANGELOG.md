# Changelog

## 0.3.0

Fixes from an audit of 0.2.0.

- Whitelist entries for names that are still defined but no longer reported
  (enum members, ignored or `# noqa` names, framework fields) are no longer
  reported as stale; entries are found from any working directory.
- Plugins tell classes apart by file: a class named like a framework model
  elsewhere no longer hides dead code. Decorator patterns (`*.get`) and
  class-name rules (`Meta`) apply in files that use the framework.
- Protocol parameters: only classes listing `Protocol` are protocols.
- Entry points use the object in their module, not every same-named one.
- A bare `yield` hides unreachable code only after `raise NotImplementedError`.
- `--format json/sarif` keeps log lines off stdout; SARIF paths are valid
  URIs and parse errors are reported in the log.
- No scan of `/` when the working directory is gone; deeply nested code no
  longer crashes; coding cookies in non-UTF-8 comments are read; cp932,
  cp949 and gb2312 are decoded.
- `condor query` rejects missing paths; the daemon does not block on a
  stalled client; the language server rejects oversized messages, checks
  its options, and handles multi-root workspaces, fast typing and syntax
  errors while typing.
- Plugins: `in_classes_based_on`; directories searched recursively; paths
  relative to the configuration file; clearer errors; a project plugin
  replaces a bundled one with the same name.
- `--config` naming a missing file, `--make-whitelist` with JSON or SARIF,
  and non-table `[tool.condor]` are errors; `--only` is case-insensitive.

Found by planting dead code in the corpus projects:

- Dead attributes, methods and parameters in a class were hidden when
  another class with the same name, in the same file or re-exported by a
  project package, was a framework class.
- With an entry point in a file, plugin rules removed the wrong findings in
  that file (a used field reported, or dead code hidden).
- `ClassVar` attributes of dataclasses and models are no longer taken for
  fields.
- Django rules follow Django's own classes: model and form fields are
  attributes assigned a call (`name = models.CharField()`), so other class
  attributes (a dead constant in a model) are reported; views, admin
  classes, inlines, list filters, admin sites, commands, app configs and
  migrations keep only the options Django defines, and the hooks Django
  calls (taken from Django 5.1's source) are used with their parameters.
  Custom user models (`AbstractUser`), `Meta` classes, `@admin.register`
  classes, form `clean_<field>` methods and signal receiver parameters are
  covered.
- attrs: plain class attributes of attrs classes are reported; fields are
  the annotated ones and those assigned `attr.ib()`/`attrs.field()`.
- Plugins: `fields = "calls"` and an `attributes` selector for `fields_used`.
- A module-level `__getattr__` or `__dir__` (PEP 562) is used.
- Test case classes (`unittest.TestCase`, Django test cases) are used
  whatever their name (plugin `unittest`).

## 0.2.0

- `condor server`: language server for editors.
- `condor query`: whole-project report through a background daemon.
- Faster directory walk.

## 0.1.0

- First release: vulture 2.16 behaviour, improvements I1 to I11, plugins,
  JSON and SARIF output.
