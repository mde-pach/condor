# Editors and agents

Two commands keep the analysis in memory, so after the first scan only
changed files are scanned again. Both take the usual options and read
`pyproject.toml`; the project is the configured `paths`, or `.`.

## Editors: `condor server`

A language server on stdin and stdout. Unused code shows as faded warnings
across the workspace, updated as you type, with a quick fix that adds
`# noqa: <code>`. Each workspace folder is analysed as its own project.
Syntax errors are left to your other tools.

Neovim (0.11+):

```lua
vim.lsp.config("condor", {
  cmd = { "condor", "server" },
  filetypes = { "python" },
  root_markers = { "pyproject.toml", ".git" },
})
vim.lsp.enable("condor")
```

Helix (`languages.toml`; keep your other servers in the list):

```toml
[language-server.condor]
command = "condor"
args = ["server"]

[[language]]
name = "python"
language-servers = ["condor"]
```

Any editor with a generic LSP client works the same way. There is no VS Code
extension yet.

## Agents and scripts: `condor query`

```console
$ condor query                 # the whole project, as `condor .` prints it
$ condor query src/app.py      # whole-project analysis, findings in this file
```

Output and exit status are those of `condor`. The first call starts a
background daemon for the directory. Later calls ask it and only rescan the
files that changed, which takes about half the time of a full scan on a
project the size of Django. A file is rescanned when its size, modification
time or (on Linux and macOS) change time changes. A path that does not exist
is an error. The daemon exits after 30 minutes without queries
(`CONDOR_DAEMON_IDLE`, in seconds). If it cannot start, or on Windows,
`condor query` scans in-process.

A Claude Code hook that shows the agent the dead code in each Python file it
edits (`.claude/settings.json`):

```json
{
  "hooks": {
    "PostToolUse": [{
      "matcher": "Edit|Write",
      "hooks": [{
        "type": "command",
        "command": "f=$(jq -r '.tool_input.file_path // empty'); case \"$f\" in *.py) condor query \"$f\" >&2; [ $? -ne 3 ] || exit 2;; esac"
      }]
    }]
  }
}
```
