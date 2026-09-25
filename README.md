<h1 align="center">
  <br>
  ⚡ ContextZip
  <br>
</h1>

<h3 align="center">
  CLI output eats your AI context window. ContextZip compresses it 40-97% (61% avg across 102 tests).<br>
  <code>cargo install --git https://github.com/150ftw/contextzip</code>
</h3>

<p align="center"><sub><b>For:</b> anyone whose AI coding agent runs terminal commands: Claude Code, Cursor, VS Code Copilot, Gemini CLI, Antigravity, OpenCode. Any model: Claude, Gemini, GPT, GPT-OSS, open-weight.<br>
<b>Not for:</b> projects where you need raw command output (use <code>contextzip proxy &lt;cmd&gt;</code> instead).</sub></p>

<p align="center">
  <a href="https://github.com/150ftw/contextzip/releases"><img src="https://img.shields.io/github/v/release/150ftw/contextzip?style=flat-square&color=blue" alt="Release" /></a>
  <a href="https://github.com/150ftw/contextzip/actions"><img src="https://img.shields.io/github/actions/workflow/status/150ftw/contextzip/ci.yml?style=flat-square" alt="CI" /></a>
  <img src="https://img.shields.io/badge/tests-1%2C132_passing-brightgreen?style=flat-square" alt="Tests" />
  <img src="https://img.shields.io/badge/benchmarks-102_cases-orange?style=flat-square" alt="Benchmarks" />
  <a href="LICENSE"><img src="https://img.shields.io/github/license/150ftw/contextzip?style=flat-square" alt="License" /></a>
</p>

---

## ⬇️ Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/150ftw/contextzip/main/install.sh | bash

contextzip init -g                  # Claude Code
contextzip init --agent all         # Cursor, VS Code Copilot, Gemini CLI, Antigravity
```

Restart your agent app. Every command is now compressed. Zero config.
**macOS · Linux · Windows**

### Works with any agent, any model

ContextZip shrinks command output, so it helps whichever model reads it: Claude, Gemini, GPT, GPT-OSS or a local model. What differs is how each app lets it hook in:

| App | Install | How it works |
|:---|:---|:---|
| Claude Code | `contextzip init -g` | Hook rewrites commands automatically |
| Cursor | `contextzip init --agent cursor` | `preToolUse` hook in `~/.cursor/hooks.json` rewrites commands automatically |
| VS Code (Copilot agent mode) | `contextzip init --agent copilot` | `PreToolUse` hook in `~/.copilot/hooks/` rewrites commands automatically |
| Gemini CLI | `contextzip init --agent gemini` | `BeforeTool` hook in `~/.gemini/settings.json` rewrites commands automatically |
| Antigravity | `contextzip init --agent antigravity` | Always-on rule asks the agent to prefix commands (Antigravity hooks can't rewrite commands) |
| OpenCode | `contextzip init -g --opencode` | Plugin rewrites commands automatically |
| Anything else | — | Tell the agent to prefix commands with `contextzip`, e.g. in `AGENTS.md` |

Remove with the same command plus `--uninstall`, e.g. `contextzip init --agent all --uninstall`.

> [!NOTE]
> In Cursor and VS Code, a command ContextZip rewrites is auto-approved, as it is in Claude Code (the only rewrite response those apps accept). Only known commands are rewritten (`git`, `cargo`, `ls`, test runners…); anything else, like `rm`, goes through the app's normal approval.

> [!TIP]
> Need raw output? Use `contextzip proxy <command>` to bypass all filters.
> Missing `jq`? Install it: `brew install jq` (macOS) or `apt install jq` (Linux). Required for the Claude Code hook.

<details>
<summary>Build from source</summary>

```bash
cargo install --git https://github.com/150ftw/contextzip
contextzip init -g
```

</details>

---

## 👀 See the Difference

### 💥 Node.js Error — 30 lines → 3 lines (92% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
TypeError: Cannot read properties
  of undefined (reading 'id')
    at getUserProfile (users.ts:47)
    at processAuth (auth.ts:12)
    at Layer.handle (node_modules/
      express/lib/router/layer.js:95)
    ... 25 more node_modules lines
```

</td>
<td width="50%">

**✅ After**
```
TypeError: Cannot read properties
  of undefined (reading 'id')
  → users.ts:47    getUserProfile()
  → auth.ts:12     processAuth()
  (+ 27 framework frames hidden)
💾 saved 92%
```

</td>
</tr>
</table>

### 📦 npm install — 150 lines → 3 lines (95% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
npm warn deprecated inflight@1.0.6
npm warn deprecated rimraf@3.0.2
... 45 more deprecated warnings
added 847 packages, audited 848
8 vulnerabilities (2 moderate, 6 high)
  ... 20 more lines
```

</td>
<td width="50%">

**✅ After**
```
✓ 847 packages (32s)
⚠ 8 vulnerabilities (6 high, 2 mod)
⚠ bcrypt@3.0.0: CVE-2023-31484
💾 saved 95%
```

</td>
</tr>
</table>

### 🐳 Docker Build — 50 lines → 1 line (96% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
Step 1/12 : FROM node:20-alpine
 ---> abc123def456
Step 2/12 : WORKDIR /app
 ---> Using cache
... 8 more steps with hashes
Successfully tagged my-app:latest
```

</td>
<td width="50%">

**✅ After**
```
✓ built my-app:latest (12 steps, 8 cached)
💾 saved 96%
```

</td>
</tr>
</table>

> [!NOTE]
> Currently supports legacy Docker builder output (`Step N/M` format). Docker BuildKit format passes through uncompressed.

### 🔨 TypeScript Build — 40 errors grouped (81% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
src/api/users.ts:47:5 - error TS2322:
  Type 'string' not assignable to 'number'
... 36 more identical errors
Found 40 errors in 8 files.
```

</td>
<td width="50%">

**✅ After**
```
TS2322: 'string' → 'number' (×40)
  users.ts :47, :83
  orders.ts :12, :45
  ... +6 files
💾 saved 81%
```

</td>
</tr>
</table>

### 🆕 v0.2 — Session History Compression (the one nobody else does)

Live stdout compression is table stakes. The bigger problem: your **past Claude Code session JSONL** under `~/.claude/projects/` accumulates **85.8% tool inputs/results** (measured across 6,850 messages). ContextZip is the first tool to compact that archive.

<table>
<tr>
<td width="50%">

**Before — 55 MB session, 2,475 records**

```
Read /src/main.rs        × 14 calls
Read /Cargo.toml         ×  9 calls
Bash "npm install"       ANSI noise + repeats
Bash "cargo test"        repeated lines
... 152 more repeated reads
... 43 more noisy Bash results
```

</td>
<td width="50%">

**After — `contextzip compact` + `apply`**

```
✓ ReadDedup        153 hits → references
✓ BashHistoryCompact 44 hits → filtered
57.3 MB → 53.5 MB (6.7% saved)
.bak preserved → safe rollback via expand
```

</td>
</tr>
</table>

```bash
contextzip compact <session-id>   # writes a reversible .compressed sidecar
contextzip apply   <session-id>   # atomic swap; original kept as .bak
contextzip expand  <session-id>   # roll back; sidecar preserved
```

<details>
<summary><b>More examples: Rust panic, Python, Web page, ANSI, Docker failure, Java/Go</b></summary>

**🐍 Python Traceback (72% saved)** — Framework frames (`flask`, `importlib`) hidden, your code + error message kept.

**🦀 Rust Panic (2-7% saved)** — `std::panicking`, `tokio::runtime` frames hidden, your crate frames kept.

**🌐 Web Page (73% saved)** — Nav, footer, sidebar, cookie banner, social links stripped. `<main>`/`<article>` content kept.

**🎨 ANSI/Spinners (83% saved)** — Escape codes, spinner frames, intermediate progress bars removed. Final states kept.

**🐳 Docker failure** — Failed step + 2 prior steps + error message + exit code always preserved.

**☕ Java** — Removes `java.lang.reflect`, `sun.reflect`, `org.springframework`, `org.apache` frames.

**🐹 Go** — Removes `runtime/`, `runtime.gopanic`, `runtime.main` frames.

</details>

---

## 📊 The Numbers Don't Lie

> **102 real-world tests. No cherry-picking.**

| Category | Tests | Avg Savings | 🏆 Best | 💀 Worst |
|:---------|------:|:----------:|:-------:|:-------:|
| 🐳 Docker build | 10 | **88%** | 97% | 77% |
| 🎨 ANSI/spinners | 15 | **83%** | 98% | 0% |
| 💥 Error traces | 20 | **59%** | 97% | -12% |
| 🔨 Build errors | 15 | **56%** | 90% | -10% |
| 🌐 Web pages | 15 | **43%** | 64% | 5% |
| 💻 CLI commands | 12 | **42%** | 99% | -56% |
| 📦 Package install | 15 | **39%** | 99% | 2% |

**Weighted total: 61% savings** → 326K chars in, 127K chars out

<details>
<summary>Why some rows show negative savings</summary>

Negative = output grew. Happens on tiny inputs where the filter's metadata costs more than it saves. We put the worst numbers in the table because hiding them would be dishonest. [Full benchmark →](docs/benchmark-results.md)

</details>

---

## 🆚 Why Not Just RTK?

Built on [RTK](https://github.com/rtk-ai/rtk) (28k⭐). All 34 RTK commands included. **Plus:**

| | RTK | ContextZip |
|:---|:---:|:---:|
| CLI compression (git, test, ls) | ✅ | ✅ |
| Error stacktraces (Node/Python/Rust/Go/Java) | ❌ | ✅ |
| Web page content extraction | ❌ | ✅ |
| ANSI / spinner / decoration removal | 🟡 | ✅ |
| Build error grouping (tsc/eslint/cargo) | 🟡 | ✅ |
| Package install noise (npm/pip/cargo) | ❌ | ✅ |
| Docker build compression | 🟡 | ✅ |
| Per-command savings display | ❌ | ✅ |
| Session-history compression (planned) | ❌ | 🚧 |
| 2026 toolchain coverage (uv/helm/biome/tf, planned) | ❌ | 🚧 |

---

## 🗺️ What's Next

The session-history compressor (`compact / apply / expand`) shipped in v0.2. Other in-flight tracks: AWS 8→25 subcommand expansion, more compact axes (`WritePlaceholder`, `EditDelta`), DSL polish.

**Other v0.2 tracks:**
- Upstream catch-up — rtk v0.31~0.36 fixes (git/aws/clippy/runner)
- New filters — `uv` (Python), `gradle`/`mvn` (JVM), `mise`, `helm`, `terraform`, `biome`
- Stability — test coverage for `env_cmd` / `verify_cmd` / `wget_cmd`
- DSL polish — env var substitution + per-platform filters

Short version in [`ROADMAP.md`](ROADMAP.md).

---

## 🛡️ Nothing Important Gets Lost

| | |
|:---|:---|
| 🔴 Error messages | **ALWAYS** preserved |
| 📍 File:line in build errors | **NEVER** removed |
| 🔒 Security warnings (CVE, GHSA) | **ALWAYS** kept |
| 🐳 Docker failure context | **ALWAYS** preserved |
| ⏎ Exit codes | **ALWAYS** propagated |

> [!IMPORTANT]
> ContextZip only removes **confirmed noise**. When in doubt → passthrough.

<details>
<summary><b>🏎️ How It Works</b></summary>

```
  ┌─────────────────────────────────────────────┐
  │  Claude Code runs: git status               │
  │                         ↓                   │
  │  Hook rewrites → contextzip git status      │
  │                         ↓                   │
  │  ┌──────────────────────────────────────┐   │
  │  │ [1] ANSI preprocessor    strip junk  │   │
  │  │ [2] Command router    40+ filters    │   │
  │  │ [3] Error post-proc   compress stack │   │
  │  │ [4] SQLite tracker    record savings │   │
  │  └──────────────────────────────────────┘   │
  │                         ↓                   │
  │  Compressed output → Claude's context       │
  │  💾 contextzip: 200 → 40 tokens (80%)       │
  └─────────────────────────────────────────────┘
```

</details>

<details>
<summary><b>🔧 Commands</b></summary>

```bash
# Automatic (hook rewrites — no prefix needed):
git status    npm install    cargo test    docker build .

# Manual:
contextzip web https://docs.example.com    # page → content only
contextzip err node server.js              # error-focused output

# Analytics:
contextzip gain                  # dashboard
contextzip gain --by-feature     # per-filter stats
contextzip gain --graph          # daily chart

# Manage:
contextzip init --show    contextzip update    contextzip uninstall
```

</details>

<details>
<summary><b>📈 Track Everything</b></summary>

```bash
$ contextzip gain
📊 ContextZip Token Savings
════════════════════════════════════════
Total commands:    2,927
Tokens saved:      10.3M (89.2%)

$ contextzip gain --by-feature
Feature        Commands  Saved     Avg%
cli (RTK)      2,100     6.8M     78%
error          89        1.2M     93%
web            43        0.9M     73%
build          112       0.4M     81%
```

</details>

---

## 🤝 Contribute

```bash
git clone https://github.com/150ftw/contextzip.git && cd contextzip
cargo test         # 1,120 tests
cargo clippy       # lint
```

Working on a track from the [roadmap](ROADMAP.md)? Open an issue first so we can confirm scope and avoid double work.

## 📡 Telemetry

Telemetry is off by default: it only runs in builds compiled with `CONTEXTZIP_TELEMETRY_URL` set, and sends anonymous counts (commands run, savings percentage), never command content.

**Disable in such a build:** `export CONTEXTZIP_TELEMETRY_DISABLED=1`

## 📜 License

MIT © Shivam Sharma. Built on [RTK](https://github.com/rtk-ai/rtk) by rtk-ai.

---

<p align="center"><b>⚡ Less noise. More code. Ship faster.</b></p>

