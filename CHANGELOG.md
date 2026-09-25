# Changelog

All notable changes to ContextZip will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.1 (2026-09-25)

### Bug Fixes

* **git diff:** never drop changed lines. `git diff`, `git show`, `git stash show` and `gh pr diff` used to cut each hunk after 30 lines and the whole diff after 500, hiding real code changes from the agent. Now every `+`/`-` line is kept; only unchanged context (trimmed from 3 lines to 1) and lock/minified file bodies are removed.
* **git diff:** removed lines whose text starts with `--` (e.g. SQL comments) were mistaken for file headers and not counted as changes.

## 0.3.0 (2026-09-25)

### Features

* **agents:** support Cursor, VS Code Copilot, Gemini CLI and Antigravity, so ContextZip works with any model these apps offer. `contextzip init --agent <cursor|copilot|gemini|antigravity|all>` installs (and `--uninstall` removes) the integration; `contextzip hook <format>` is the hook entry point.

## 0.2.0 (2026-09-25)

First release under this repository.

### Bug Fixes

* **wget:** rename test to snake_case so `cargo clippy -- -D warnings` passes
* **update:** replace the binary via rename instead of writing into the running executable (fixed "Text file busy" on Linux)
* **release:** publish `contextzip-<os>-<arch>.tar.gz` assets that `contextzip update` downloads
