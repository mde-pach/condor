# Changelog

## Unreleased

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

## 0.2.0

- `condor server`: language server for editors.
- `condor query`: whole-project report through a background daemon.
- Faster directory walk.

## 0.1.0

- First release: vulture 2.16 behaviour, improvements I1 to I11, plugins,
  JSON and SARIF output.
