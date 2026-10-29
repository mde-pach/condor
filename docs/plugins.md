# Plugins

Plugins tell condor which code a library uses, so it is not reported. They
are built into condor and apply only when your project imports the library.

**Included:** abc, dataclasses, enum, typing, pytest, pydantic, attrs,
msgspec, sqlmodel, Django, Celery, Flask, FastAPI, click. They live in
[condor-plugins](https://github.com/mde-pach/condor-plugins); contributions
welcome.

## Your own

Add files (TOML or JSON) or directories of them:

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

[[fields_used]]             # these class attributes are used
inherits = ["mylib.Model"]
```

| Rule | Declares used |
|---|---|
| `used` | functions, methods, classes by `decorated_by`, `names`, `inherits`, `in_classes_inheriting` |
| `fields_used` | attributes of matching classes and their subclasses (`fields`: `annotated`, `assigned`, `all`) |
| `parameters_used` | parameters of matching functions |
| `injected_by_name` | values passed by parameter name, like pytest fixtures |

Names are written as imported (`pydantic.BaseModel`) and accept `*`
wildcards. The full format is in
[schema.json](https://github.com/mde-pach/condor-plugins/blob/main/schema.json).
