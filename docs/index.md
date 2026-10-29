# condor

Finds unused Python code: functions nobody calls, classes nobody uses,
leftover imports and variables, unreachable code. A faster
[vulture](https://github.com/jendrikseipp/vulture) with fewer false positives.

<pre><code>$ uvx cndr src/
src/app/loader.py:12: <span class="condor-finding">unused function 'useful_load'</span> (60% confidence)
src/app/models.py:3: <span class="condor-finding">unused import 'json'</span> (90% confidence)
src/app/worker.py:41: <span class="condor-finding">unreachable code after 'return'</span> (100% confidence)
</code></pre>

## Install

The package is `cndr`; it installs the `condor` and `cndr` commands.

<div class="grid cards" markdown>

-   <span class="condor-card-label">Run once</span>

    `uvx cndr .`

-   <span class="condor-card-label">Install the tool</span>

    `uv tool install cndr`

-   <span class="condor-card-label">In a project</span>

    `uv add --dev cndr`

</div>

## Use

```console
$ condor src/ tests/                # files or directories
$ condor . --min-confidence 80      # only the surest findings
$ condor . --only function,class    # only some kinds
$ condor . --format json            # or sarif
$ condor . --make-whitelist > whitelist.py
```

Exit status: `0` nothing found, `3` dead code found, `1` a file could not be
parsed, `2` bad options. `condor --help` lists every option.

## Configure

In `pyproject.toml` (`[tool.vulture]` works too):

```toml
[tool.condor]
paths = ["src", "tests"]
exclude = ["*/migrations/*"]
ignore_names = ["visit_*"]
min_confidence = 60
```

## False positives

Code used by a framework or by name looks unused. condor knows the common
cases through [plugins](plugins.md). For the rest: add `# noqa` to the line,
add the name to `ignore_names`, or keep a whitelist made with
`--make-whitelist` and pass it as a path.
