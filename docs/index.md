<section class="ch-hero ch-play" aria-labelledby="hero-name">
<div class="ch-lockup">
<svg class="ch-mark" viewBox="0 0 200 200" aria-hidden="true"><g transform="translate(10.5 2)" fill="none" stroke="currentColor" stroke-width="10" stroke-linejoin="round" stroke-linecap="round"><path d="M150 170 C156 140 158 110 150 88 C142 66 124 56 104 58 C108 52 110 46 106 40 C100 30 82 26 66 32 C46 40 40 66 47.5 86.2 C42.6 88.2 38 90.8 34 94 C24 102 21 116 25 128 C27 134 31 138 36 140 C35 131 37 123 44 119 C52 115 62 115 72 117 C86 124 94 146 96 170"/></g><circle cx="122.5" cy="90" r="8" fill="currentColor"/></svg>
<div class="ch-text">
<h1 class="ch-word" id="hero-name" aria-label="condor, or cndr"><span>c</span><span class="ch-o"><span class="ch-glyph">o</span></span><span>nd</span><span class="ch-o"><span class="ch-glyph">o</span></span><span>r</span></h1>
<div class="ch-report" aria-hidden="true">
<div class="ch-l1">condor:1:2: <em>unused letter 'o'</em> (100% confidence)</div>
<div class="ch-l2">condor:1:5: <em>unused letter 'o'</em> (100% confidence)</div>
</div>
</div>
</div>
<p class="ch-tag">Finds unused Python code: functions nobody calls, classes nobody uses, leftover imports and unreachable code. A faster vulture with fewer false positives.</p>
<div class="ch-actions">
<span class="ch-cmd"><code>uvx cndr .</code><button type="button">Copy</button></span>
<a class="ch-start" href="#install">Get started</a>
<button type="button" class="ch-replay">Replay</button>
</div>
</section>

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

## Speed

![Scanning Django: condor versus vulture](assets/bench-dark.svg#only-dark)
![Scanning Django: condor versus vulture](assets/bench-light.svg#only-light)

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
