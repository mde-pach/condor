# Plugins

Plugins tell condor which code a library uses, so it is not reported. They
are built into condor and apply only when your project imports the library.

**Included:** abc, dataclasses, enum, typing, unittest, pytest, pydantic, attrs,
msgspec, sqlmodel, Django, Celery, Flask, FastAPI, click. They live in
[condor-plugins](https://github.com/mde-pach/condor-plugins); contributions
welcome.

## Your own

Add files (TOML or JSON) or directories of them, relative to the
configuration file. A plugin named like an included one replaces it.

```toml
[tool.condor]
plugins = ["condor-rules"]
```

```toml
# condor-rules/mylib.toml
[plugin]
name = "mylib"
when_imported = ["mylib"]

[[used]]                    # these definitions are used
decorated_by = ["mylib.handler", "*.route"]

[[fields_used]]             # annotated attributes of these classes are used
inherits = ["mylib.Model"]
fields = "annotated"        # or "assigned" (x = 1), "calls" (x = f()), "all"
```

| Rule | Declares used |
|---|---|
| `used` | functions, methods, classes matching all of `decorated_by`, `names`, `inherits`, `in_classes_inheriting`, `in_classes_based_on` |
| `fields_used` | attributes of classes matching any of `decorated_by`, `inherits`, `names`, and of their subclasses (`fields`: `annotated` by default, `assigned`, `calls` for attributes assigned a call such as `models.CharField()`, `all`; `attributes` limits the rule to these names) |
| `parameters_used` | parameters of functions matching all of `decorated_by`, `names`, `in_classes_inheriting`, `in_classes_based_on` |
| `injected_by_name` | values passed by parameter name, like pytest fixtures |
| `variables_used` | module and class variables read by name, like `pytestmark` |

Names are written as imported (`pydantic.BaseModel`) and accept `*`
wildcards. Classes are followed through each file's imports, so a class with
the same name elsewhere does not match; `in_classes_based_on` looks only at a
class's own bases (a `Protocol`, not its implementations). Decorator patterns
and class names apply in files that import a `when_imported` module. The full format is in
[schema.json](https://github.com/mde-pach/condor-plugins/blob/main/schema.json).
