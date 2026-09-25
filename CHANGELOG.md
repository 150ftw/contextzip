# Changelog

All notable changes to ContextZip will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.0 (2026-09-25)

### Features

* **agents:** support Cursor, VS Code Copilot, Gemini CLI and Antigravity, so ContextZip works with any model these apps offer. `contextzip init --agent <cursor|copilot|gemini|antigravity|all>` installs (and `--uninstall` removes) the integration; `contextzip hook <format>` is the hook entry point.

## 0.2.0 (2026-09-25)

First release under this repository.

### Bug Fixes

* **wget:** rename test to snake_case so `cargo clippy -- -D warnings` passes
* **update:** replace the binary via rename instead of writing into the running executable (fixed "Text file busy" on Linux)
* **release:** publish `contextzip-<os>-<arch>.tar.gz` assets that `contextzip update` downloads
