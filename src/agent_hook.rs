//! Integrations for AI coding agents other than Claude Code.
//!
//! ContextZip is model-agnostic: it filters command output, so it helps any model.
//! What differs is how each agent app lets us intercept a shell command. This module
//! provides one hook entry point (`contextzip hook <format>`) that speaks each app's
//! JSON protocol, plus installers that register it in the app's config.
//!
//! | App            | Mechanism                                   | Rewrite |
//! |----------------|---------------------------------------------|---------|
//! | Cursor         | `~/.cursor/hooks.json` preToolUse (Shell)   | yes     |
//! | Gemini CLI     | `~/.gemini/settings.json` BeforeTool        | yes     |
//! | VS Code Copilot| `~/.copilot/hooks/contextzip.json` PreToolUse | yes   |
//! | Antigravity    | `~/.gemini/config/rules/contextzip.md` rule | no (hooks cannot rewrite; the rule asks the agent to prefix commands) |
//!
//! All rewrite decisions come from `discover::registry::rewrite_command`, the same
//! source of truth used by the Claude Code hook.

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Wire format spoken by `contextzip hook <format>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum HookFormat {
    /// Cursor preToolUse (tool_name "Shell", replies with updated_input)
    Cursor,
    /// Gemini CLI BeforeTool (tool_name "run_shell_command", replies with tool_input)
    Gemini,
    /// VS Code Copilot Chat PreToolUse (tool_name "run_in_terminal", replies with updatedInput)
    Copilot,
}

/// Agent app to install into or remove from with `contextzip init --agent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Agent {
    Cursor,
    Gemini,
    Copilot,
    Antigravity,
    /// Every agent above
    All,
}

impl Agent {
    fn expand(agents: &[Agent]) -> Vec<Agent> {
        let mut out = Vec::new();
        for a in agents {
            let list: &[Agent] = if *a == Agent::All {
                &[
                    Agent::Cursor,
                    Agent::Gemini,
                    Agent::Copilot,
                    Agent::Antigravity,
                ]
            } else {
                std::slice::from_ref(a)
            };
            for x in list {
                if !out.contains(x) {
                    out.push(*x);
                }
            }
        }
        out
    }

    fn label(self) -> &'static str {
        match self {
            Agent::Cursor => "Cursor",
            Agent::Gemini => "Gemini CLI",
            Agent::Copilot => "VS Code Copilot",
            Agent::Antigravity => "Antigravity",
            Agent::All => "all",
        }
    }
}

// ─── Hook runtime ────────────────────────────────────────────────────────────

/// Entry point for `contextzip hook <format>`: read the agent's JSON on stdin and
/// print a rewrite response, or nothing to let the command run unchanged.
/// Never fails: a hook error must not block the user's agent.
pub fn run(format: HookFormat) -> Result<()> {
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_err() {
        return Ok(());
    }
    let Ok(input) = serde_json::from_str::<Value>(&buf) else {
        return Ok(());
    };

    let excluded = crate::config::Config::load()
        .map(|c| c.hooks.exclude_commands)
        .unwrap_or_default();
    let rewrite = |cmd: &str| crate::discover::registry::rewrite_command(cmd, &excluded);

    if let Some(out) = respond(format, &input, &rewrite) {
        println!("{}", out);
    }
    Ok(())
}

/// Build the agent-specific response, or `None` when the command should pass through.
fn respond(
    format: HookFormat,
    input: &Value,
    rewrite: &dyn Fn(&str) -> Option<String>,
) -> Option<Value> {
    let tool = input.get("tool_name")?.as_str()?;
    let tool_input = input.get("tool_input")?.as_object()?;
    let command = tool_input.get("command")?.as_str()?;

    let shell_tools: &[&str] = match format {
        HookFormat::Cursor => &["Shell"],
        HookFormat::Gemini => &["run_shell_command"],
        HookFormat::Copilot => &["run_in_terminal", "runTerminalCommand"],
    };
    if !shell_tools.contains(&tool) {
        return None;
    }

    let rewritten = rewrite(command)?;
    if rewritten == command {
        return None;
    }

    // Cursor and VS Code replace the whole tool input (VS Code drops it if it no
    // longer matches the tool schema), so carry the other fields over unchanged.
    let mut updated = tool_input.clone();
    updated.insert("command".into(), Value::String(rewritten.clone()));

    Some(match format {
        HookFormat::Cursor => json!({
            "permission": "allow",
            "updated_input": updated,
        }),
        // Gemini merges tool_input into the model's arguments; no decision is sent,
        // so Gemini's own approval flow still applies.
        HookFormat::Gemini => json!({
            "hookSpecificOutput": {
                "hookEventName": "BeforeTool",
                "tool_input": { "command": rewritten },
            }
        }),
        // "ask" + updatedInput loops forever in VS Code; "allow" is the verified path.
        HookFormat::Copilot => json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "allow",
                "permissionDecisionReason": "ContextZip auto-rewrite",
                "updatedInput": updated,
            }
        }),
    })
}

// ─── Install / uninstall ─────────────────────────────────────────────────────

const ANTIGRAVITY_RULE: &str = r#"---
trigger: always_on
---

# ContextZip

`contextzip` is installed. It runs a command and returns a compact version of its
output (errors, failures, and results kept; noise removed), saving context.

When running shell commands, prefix these with `contextzip`:
git, gh, cargo, npm, pnpm, npx, pytest, go, tsc, eslint, docker, kubectl, ls, find,
grep, rg, diff, curl, pip, ruff, mypy. Example: `contextzip git status`.

Chained commands: prefix each part (`contextzip cargo test && contextzip git status`).
Need the full raw output? Use `contextzip proxy <command>`.
"#;

/// Entry point for `contextzip init --agent ...` (and `--uninstall`).
pub fn run_init(agents: &[Agent], uninstall: bool) -> Result<()> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let exe = hook_executable()?;
    let mut failed = false;

    for agent in Agent::expand(agents) {
        let result = if uninstall {
            uninstall_agent(agent, &home).map(|paths| {
                if paths.is_empty() {
                    format!("{}: nothing to remove", agent.label())
                } else {
                    format!("{}: removed {}", agent.label(), join_paths(&paths))
                }
            })
        } else {
            install_agent(agent, &home, &exe)
                .map(|path| format!("{}: {}", agent.label(), path.display()))
        };
        match result {
            Ok(line) => println!("[ok] {}", line),
            Err(e) => {
                failed = true;
                eprintln!("[error] {}: {:#}", agent.label(), e);
            }
        }
    }

    if !uninstall {
        println!("\nRestart the agent apps to load the change. Test by asking the agent to run `git status`.");
    }
    if failed {
        anyhow::bail!("some agents could not be configured (see errors above)");
    }
    Ok(())
}

/// Absolute path to this binary: GUI apps often launch hooks without the
/// user's shell PATH, so a bare `contextzip` may not resolve.
fn hook_executable() -> Result<String> {
    let exe = std::env::current_exe().context("Cannot locate the contextzip binary")?;
    let exe = exe.canonicalize().unwrap_or(exe);
    Ok(exe.to_string_lossy().into_owned())
}

fn hook_command(exe: &str, format: &str) -> String {
    format!("\"{}\" hook {}", exe, format)
}

/// True for any hook command this module wrote, whatever binary path it used.
fn is_our_command(cmd: &str, format: &str) -> bool {
    cmd.contains("contextzip") && cmd.trim_end().ends_with(&format!("hook {}", format))
}

fn install_agent(agent: Agent, home: &Path, exe: &str) -> Result<PathBuf> {
    match agent {
        Agent::Cursor => {
            let path = home.join(".cursor").join("hooks.json");
            let mut root = read_json_or(&path, json!({ "version": 1, "hooks": {} }))?;
            let list = json_array_at(&mut root, &["hooks", "preToolUse"])?;
            list.retain(|e| !entry_command_matches(e, "cursor"));
            list.push(json!({ "command": hook_command(exe, "cursor"), "matcher": "Shell" }));
            write_json(&path, &root)?;
            Ok(path)
        }
        Agent::Gemini => {
            let path = home.join(".gemini").join("settings.json");
            let mut root = read_json_or(&path, json!({}))?;
            let list = json_array_at(&mut root, &["hooks", "BeforeTool"])?;
            list.retain(|g| !group_has_our_hook(g, "gemini"));
            list.push(json!({
                "matcher": "run_shell_command",
                "hooks": [{
                    "name": "contextzip",
                    "type": "command",
                    "command": hook_command(exe, "gemini"),
                    "timeout": 5000,
                }]
            }));
            write_json(&path, &root)?;
            Ok(path)
        }
        Agent::Copilot => {
            let path = home.join(".copilot").join("hooks").join("contextzip.json");
            let root = json!({
                "hooks": {
                    "PreToolUse": [{
                        "type": "command",
                        "command": hook_command(exe, "copilot"),
                        "timeout": 10,
                    }]
                }
            });
            write_json(&path, &root)?;
            Ok(path)
        }
        Agent::Antigravity => {
            let path = antigravity_rule_path(home);
            write_file(&path, ANTIGRAVITY_RULE)?;
            Ok(path)
        }
        Agent::All => unreachable!("expanded before install"),
    }
}

fn uninstall_agent(agent: Agent, home: &Path) -> Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    match agent {
        Agent::Cursor => {
            let path = home.join(".cursor").join("hooks.json");
            if path.exists() {
                let mut root = read_json_or(&path, json!({}))?;
                if let Some(list) = root
                    .pointer_mut("/hooks/preToolUse")
                    .and_then(Value::as_array_mut)
                {
                    let before = list.len();
                    list.retain(|e| !entry_command_matches(e, "cursor"));
                    if list.len() != before {
                        write_json(&path, &root)?;
                        removed.push(path);
                    }
                }
            }
        }
        Agent::Gemini => {
            let path = home.join(".gemini").join("settings.json");
            if path.exists() {
                let mut root = read_json_or(&path, json!({}))?;
                if let Some(list) = root
                    .pointer_mut("/hooks/BeforeTool")
                    .and_then(Value::as_array_mut)
                {
                    let before = list.len();
                    list.retain(|g| !group_has_our_hook(g, "gemini"));
                    if list.len() != before {
                        write_json(&path, &root)?;
                        removed.push(path);
                    }
                }
            }
        }
        Agent::Copilot => {
            let path = home.join(".copilot").join("hooks").join("contextzip.json");
            if path.exists() {
                fs::remove_file(&path)
                    .with_context(|| format!("Failed to remove {}", path.display()))?;
                removed.push(path);
            }
        }
        Agent::Antigravity => {
            let path = antigravity_rule_path(home);
            if path.exists() {
                fs::remove_file(&path)
                    .with_context(|| format!("Failed to remove {}", path.display()))?;
                removed.push(path);
            }
        }
        Agent::All => unreachable!("expanded before uninstall"),
    }
    Ok(removed)
}

fn antigravity_rule_path(home: &Path) -> PathBuf {
    home.join(".gemini")
        .join("config")
        .join("rules")
        .join("contextzip.md")
}

fn entry_command_matches(entry: &Value, format: &str) -> bool {
    entry
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| is_our_command(c, format))
}

fn group_has_our_hook(group: &Value, format: &str) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| hooks.iter().any(|h| entry_command_matches(h, format)))
}

// ─── JSON file helpers ───────────────────────────────────────────────────────

/// Read a JSON config, or return `default` if the file is missing or empty.
/// Refuses to touch a file it cannot parse, so user config is never clobbered.
fn read_json_or(path: &Path, default: Value) -> Result<Value> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(default),
        Err(e) => {
            return Err(e).with_context(|| format!("Failed to read {}", path.display()));
        }
    };
    if content.trim().is_empty() {
        return Ok(default);
    }
    serde_json::from_str(&content).with_context(|| {
        format!(
            "{} is not plain JSON (comments?); left it untouched — add the hook manually",
            path.display()
        )
    })
}

/// Walk `keys` from `root`, creating objects as needed, and return the array at the end.
fn json_array_at<'a>(root: &'a mut Value, keys: &[&str]) -> Result<&'a mut Vec<Value>> {
    let (last, parents) = keys.split_last().context("empty key path")?;
    let mut node = root;
    for key in parents {
        let obj = node
            .as_object_mut()
            .with_context(|| format!("expected an object above `{}`", key))?;
        node = obj
            .entry(key.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    let obj = node
        .as_object_mut()
        .with_context(|| format!("expected an object above `{}`", last))?;
    obj.entry(last.to_string())
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .with_context(|| format!("`{}` is not a list", last))
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value).context("Failed to serialize JSON")?;
    write_file(path, &(text + "\n"))
}

/// Write via a sibling temp file + rename so a crash never leaves a half-written config.
fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("Failed to create {}", dir.display()))?;
    }
    let tmp = path.with_extension("contextzip-tmp");
    fs::write(&tmp, content).with_context(|| format!("Failed to write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("Failed to replace {}", path.display()))
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn real_rewrite(cmd: &str) -> Option<String> {
        crate::discover::registry::rewrite_command(cmd, &[])
    }

    // Payloads below mirror the documented / observed stdin of each app.

    #[test]
    fn cursor_rewrites_shell_and_keeps_other_fields() {
        let input = json!({
            "tool_name": "Shell",
            "tool_input": { "command": "git status", "working_directory": "/proj" },
            "cwd": "/proj"
        });
        let out = respond(HookFormat::Cursor, &input, &real_rewrite).expect("rewrite");
        assert_eq!(out["permission"], "allow");
        assert_eq!(out["updated_input"]["command"], "contextzip git status");
        assert_eq!(out["updated_input"]["working_directory"], "/proj");
    }

    #[test]
    fn gemini_rewrites_run_shell_command() {
        let input = json!({
            "hook_event_name": "BeforeTool",
            "tool_name": "run_shell_command",
            "tool_input": { "command": "cargo test", "description": "run tests" }
        });
        let out = respond(HookFormat::Gemini, &input, &real_rewrite).expect("rewrite");
        assert_eq!(out["hookSpecificOutput"]["hookEventName"], "BeforeTool");
        assert_eq!(
            out["hookSpecificOutput"]["tool_input"]["command"],
            "contextzip cargo test"
        );
        assert!(
            out.get("decision").is_none(),
            "must not bypass Gemini approval"
        );
    }

    #[test]
    fn copilot_rewrites_run_in_terminal_with_full_schema() {
        let input = json!({
            "tool_name": "run_in_terminal",
            "tool_input": {
                "command": "git status",
                "explanation": "Check current git status",
                "mode": "sync",
                "timeout": 60000
            }
        });
        let out = respond(HookFormat::Copilot, &input, &real_rewrite).expect("rewrite");
        let hso = &out["hookSpecificOutput"];
        assert_eq!(hso["permissionDecision"], "allow");
        assert_eq!(hso["updatedInput"]["command"], "contextzip git status");
        assert_eq!(
            hso["updatedInput"]["explanation"],
            "Check current git status"
        );
        assert_eq!(hso["updatedInput"]["mode"], "sync");
        assert_eq!(hso["updatedInput"]["timeout"], 60000);
    }

    #[test]
    fn unsupported_command_passes_through() {
        let input = json!({ "tool_name": "Shell", "tool_input": { "command": "rm -rf build" } });
        assert!(respond(HookFormat::Cursor, &input, &real_rewrite).is_none());
    }

    #[test]
    fn already_prefixed_command_passes_through() {
        let input = json!({
            "tool_name": "run_shell_command",
            "tool_input": { "command": "contextzip git status" }
        });
        assert!(respond(HookFormat::Gemini, &input, &real_rewrite).is_none());
    }

    #[test]
    fn non_shell_tool_passes_through() {
        let input = json!({ "tool_name": "Read", "tool_input": { "command": "git status" } });
        assert!(respond(HookFormat::Cursor, &input, &real_rewrite).is_none());
        let input = json!({ "tool_name": "Shell", "tool_input": { "command": "git status" } });
        assert!(respond(HookFormat::Gemini, &input, &real_rewrite).is_none());
    }

    #[test]
    fn malformed_payload_passes_through() {
        for input in [
            json!({}),
            json!({ "tool_name": "Shell" }),
            json!({ "tool_name": "Shell", "tool_input": "git status" }),
            json!({ "tool_name": "Shell", "tool_input": { "command": 42 } }),
        ] {
            assert!(respond(HookFormat::Cursor, &input, &real_rewrite).is_none());
        }
    }

    #[test]
    fn expand_all_lists_each_agent_once() {
        let agents = Agent::expand(&[Agent::Cursor, Agent::All]);
        assert_eq!(
            agents,
            vec![
                Agent::Cursor,
                Agent::Gemini,
                Agent::Copilot,
                Agent::Antigravity
            ]
        );
    }

    #[test]
    fn cursor_install_preserves_existing_hooks_and_is_idempotent() {
        let home = TempDir::new().expect("tempdir");
        let path = home.path().join(".cursor/hooks.json");
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &path,
            r#"{"version":1,"hooks":{"preToolUse":[{"command":"./audit.sh"}],"stop":[{"command":"./s.sh"}]}}"#,
        )
        .expect("seed");

        install_agent(Agent::Cursor, home.path(), "/bin/contextzip").expect("install 1");
        install_agent(Agent::Cursor, home.path(), "/bin/contextzip").expect("install 2");

        let root: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        let list = root["hooks"]["preToolUse"].as_array().expect("array");
        assert_eq!(list.len(), 2, "user hook kept, ours added once");
        assert_eq!(list[0]["command"], "./audit.sh");
        assert_eq!(list[1]["command"], "\"/bin/contextzip\" hook cursor");
        assert_eq!(list[1]["matcher"], "Shell");
        assert_eq!(root["hooks"]["stop"][0]["command"], "./s.sh");

        let removed = uninstall_agent(Agent::Cursor, home.path()).expect("uninstall");
        assert_eq!(removed.len(), 1);
        let root: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(
            root["hooks"]["preToolUse"].as_array().expect("array").len(),
            1
        );
    }

    #[test]
    fn gemini_install_keeps_other_settings() {
        let home = TempDir::new().expect("tempdir");
        let path = home.path().join(".gemini/settings.json");
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, r#"{"theme":"Dracula","hooks":{"AfterTool":[]}}"#).expect("seed");

        install_agent(Agent::Gemini, home.path(), "/bin/contextzip").expect("install");
        install_agent(Agent::Gemini, home.path(), "/bin/contextzip").expect("reinstall");

        let root: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(root["theme"], "Dracula");
        assert!(root["hooks"]["AfterTool"].is_array());
        let groups = root["hooks"]["BeforeTool"].as_array().expect("array");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["matcher"], "run_shell_command");
        assert_eq!(
            groups[0]["hooks"][0]["command"],
            "\"/bin/contextzip\" hook gemini"
        );

        uninstall_agent(Agent::Gemini, home.path()).expect("uninstall");
        let root: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(root["theme"], "Dracula");
        assert!(root["hooks"]["BeforeTool"]
            .as_array()
            .expect("array")
            .is_empty());
    }

    #[test]
    fn unparseable_config_is_left_untouched() {
        let home = TempDir::new().expect("tempdir");
        let path = home.path().join(".gemini/settings.json");
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        let original = "{ // comment\n \"theme\": \"x\" }";
        fs::write(&path, original).expect("seed");

        assert!(install_agent(Agent::Gemini, home.path(), "/bin/contextzip").is_err());
        assert_eq!(fs::read_to_string(&path).expect("read"), original);
    }

    #[test]
    fn copilot_and_antigravity_files_round_trip() {
        let home = TempDir::new().expect("tempdir");

        let copilot =
            install_agent(Agent::Copilot, home.path(), "/bin/contextzip").expect("copilot install");
        let root: Value =
            serde_json::from_str(&fs::read_to_string(&copilot).expect("read")).expect("json");
        assert_eq!(
            root["hooks"]["PreToolUse"][0]["command"],
            "\"/bin/contextzip\" hook copilot"
        );

        let rule = install_agent(Agent::Antigravity, home.path(), "/bin/contextzip")
            .expect("antigravity install");
        let text = fs::read_to_string(&rule).expect("read");
        assert!(text.starts_with("---\ntrigger: always_on\n---"));

        assert_eq!(
            uninstall_agent(Agent::Copilot, home.path())
                .expect("rm")
                .len(),
            1
        );
        assert_eq!(
            uninstall_agent(Agent::Antigravity, home.path())
                .expect("rm")
                .len(),
            1
        );
        assert!(!copilot.exists() && !rule.exists());
        assert!(uninstall_agent(Agent::Copilot, home.path())
            .expect("rm again")
            .is_empty());
    }
}
