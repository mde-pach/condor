# Plugins

Libraries call code in ways a name-based analysis cannot see: pydantic reads
model fields, pytest passes fixtures by parameter name, Django calls `save()`
and registers template filters through decorators. condor learns these from
plugins: declarative rule files, one per library, that say which definitions
are used. Plugins are data, not code; condor interprets them.

## Where plugins come from

* **Bundled.** condor embeds a pinned snapshot of
  [condor-plugins](https://github.com/mde-pach/condor-plugins) in
  `crates/condor_python/plugins/` (`SOURCE` records the revision). Update it
  with `scripts/sync-plugins.sh PATH_TO_CONDOR_PLUGINS`. Rules are never
  downloaded at run time, so results do not change between runs.
* **The project's own.** `[tool.condor] plugins = ["condor/rules"]` adds plugin
  files, or directories of them, relative to the current directory.

A plugin applies when the project imports one of the modules in its
`when_imported` list (any file, any import form), or always if the list is
empty.

## Format

TOML or JSON, with the same schema
([schema.json](https://github.com/mde-pach/condor-plugins/blob/main/schema.json),
usable for editor validation of both formats).

```toml
[plugin]
name = "mylib"
description = "Why this code is used."
when_imported = ["mylib"]

[[used]]                       # definitions that are used
decorated_by = ["mylib.handler", "*.route"]

[[used]]                       # methods the framework calls
in_classes_inheriting = ["mylib.Base"]
names = ["setup", "run"]

[[fields_used]]                # class attributes that are used
inherits = ["mylib.Model"]
fields = "annotated"           # annotated (x: int) | assigned (X = 1) | all

[[parameters_used]]            # parameters imposed by a signature
decorated_by = ["mylib.callback"]

[[injected_by_name]]           # values passed by parameter name
providers = { decorated_by = ["mylib.provider"] }
name_keyword = "name"          # @provider(name="x") is requested as x
always_used_keyword = "auto"   # @provider(auto=True) is always used
consumers = { names = ["test*"], files = ["test_*.py"] }
requested_by_calls = ["mylib.uses"]   # mylib.uses("x") requests x
```

| Rule | Selects | Declares used |
|---|---|---|
| `used` | functions and methods by `decorated_by`, `names`, `in_classes_inheriting`; classes by `decorated_by`, `names`, `inherits` | the selected definitions |
| `fields_used` | classes by `decorated_by`, `inherits` or `names`, and their subclasses in the project | their annotated and/or assigned attributes |
| `parameters_used` | functions by `decorated_by`, `names`, `in_classes_inheriting` | their parameters |
| `injected_by_name` | providers by `decorated_by` or `names`; consumers by function `names` and `files` | providers requested by a consumer's or provider's parameter, by a `requested_by_calls` call, or always used; and the parameters of consumers and providers |

In a `used` rule, every selector given must match.

### Names

Selectors are dotted names as the library defines them (`pydantic.BaseModel`),
with glob wildcards (`*`, `?`, `[abc]`). condor resolves names in the code
through each file's imports: `from pydantic import BaseModel`,
`import pydantic`, and `import pydantic as pd` all lead to
`pydantic.BaseModel`. Decorators on local objects cannot be resolved and keep
their written name, hence patterns like `register.filter` or `*.route`. Names
brought in by star imports are not resolved and do not match.

Inheritance follows subclasses defined anywhere in the project:
`class Base(BaseModel)` in one file makes `class User(Base)` in another a
pydantic model too.
