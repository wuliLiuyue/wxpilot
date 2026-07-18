// SPDX-License-Identifier: AGPL-3.0-or-later
// wxpilot - 面向 AI Agent 的微信小程序自动化 CLI
// Copyright (C) 2026  wuliLiuyue
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{json, Map, Value};
use wxp_common::{DAEMON_KIND, ERROR_REF_EXPIRED, RUST_PID_FILE, RUST_SOCKET_FILE};

use crate::command::CliCommand;
use crate::daemon_client::{connect_to_daemon, stop_daemon, ConnectDeps, ProbeResult, StopDeps};
use crate::error::CliError;
use crate::project_detect::{detect_or_select_project, Selector};
use crate::rpc_client::call_with_timeout;

#[derive(Debug)]
pub struct RpcCall {
    pub method: &'static str,
    pub params: Value,
    pub timeout_ms: u64,
}

pub fn build_rpc_call(command: &CliCommand) -> Result<RpcCall, CliError> {
    match command {
        CliCommand::Start {
            project_path,
            cli_path,
            appid,
            auto_port,
            timeout_ms,
            ..
        } => {
            let project_path = project_path.clone().ok_or_else(|| {
                CliError::Message("start command requires resolved project path".into())
            })?;
            let mut params = Map::new();
            params.insert(String::from("projectPath"), Value::String(project_path));
            if let Some(cli_path) = cli_path {
                params.insert(String::from("cliPath"), Value::String(cli_path.clone()));
            }
            if let Some(appid) = appid {
                params.insert(String::from("appid"), Value::String(appid.clone()));
            }
            params.insert(String::from("port"), json!(auto_port));
            Ok(RpcCall {
                method: "start",
                params: Value::Object(params),
                timeout_ms: *timeout_ms,
            })
        }
        CliCommand::Connect {
            ws_endpoint,
            timeout_ms,
        } => {
            let params = ws_endpoint
                .as_ref()
                .map(|ws_endpoint| json!({ "wsEndpoint": ws_endpoint }))
                .unwrap_or_else(|| json!({}));
            Ok(RpcCall {
                method: "connect",
                params,
                timeout_ms: *timeout_ms,
            })
        }
        CliCommand::Disconnect => Ok(simple_call("disconnect")),
        CliCommand::Status => Ok(simple_call("status")),
        CliCommand::Goto { route, timeout_ms } => Ok(RpcCall {
            method: "goto",
            params: json!({ "route": route }),
            timeout_ms: *timeout_ms,
        }),
        CliCommand::Back => Ok(simple_call("back")),
        CliCommand::Reload => Ok(simple_call("reload")),
        CliCommand::View {
            full,
            compact,
            depth,
            limit,
        } => Ok(RpcCall {
            method: "view",
            params: optional_object([
                ("full", Some(json!(full))),
                ("compact", compact.as_ref().map(|value| Value::from(*value))),
                ("depth", depth.as_ref().map(|value| Value::from(*value))),
                ("limit", limit.as_ref().map(|value| Value::from(*value))),
            ]),
            timeout_ms: 10_000,
        }),
        CliCommand::Find { query, all, limit } => Ok(RpcCall {
            method: "find",
            params: json!({
                "query": query,
                "all": all,
                "limit": limit,
            }),
            timeout_ms: 10_000,
        }),
        CliCommand::Wait {
            reference,
            timeout_ms,
        } => Ok(RpcCall {
            method: "wait",
            params: json!({
                "ref": parse_wait_ref(reference)?,
                "timeout": timeout_ms,
            }),
            timeout_ms: *timeout_ms,
        }),
        CliCommand::Tap { reference } => Ok(RpcCall {
            method: "tap",
            params: json!({ "ref": parse_ref(reference)? }),
            timeout_ms: 10_000,
        }),
        CliCommand::Type { reference, text } => Ok(RpcCall {
            method: "type",
            params: json!({ "ref": parse_ref(reference)?, "text": text }),
            timeout_ms: 10_000,
        }),
        CliCommand::Scroll {
            reference,
            direction,
        } => {
            if direction != "up" && direction != "down" {
                return Err(CliError::Message("direction must be up or down".into()));
            }
            Ok(RpcCall {
                method: "scroll",
                params: json!({ "ref": parse_ref(reference)?, "direction": direction }),
                timeout_ms: 10_000,
            })
        }
        CliCommand::Read { reference } => Ok(RpcCall {
            method: "read",
            params: json!({ "ref": parse_ref(reference)? }),
            timeout_ms: 10_000,
        }),
        CliCommand::Assert {
            reference,
            expected,
        } => Ok(RpcCall {
            method: "assert",
            params: json!({ "ref": parse_ref(reference)?, "expected": expected }),
            timeout_ms: 10_000,
        }),
        CliCommand::State { path, full } => Ok(RpcCall {
            method: "state",
            params: json!({ "path": path, "full": full }),
            timeout_ms: 10_000,
        }),
        CliCommand::Shot { path, base64 } => Ok(RpcCall {
            method: "shot",
            params: json!({ "path": path, "base64": base64 }),
            timeout_ms: 10_000,
        }),
        CliCommand::Run { js } => Ok(RpcCall {
            method: "run",
            params: json!({ "js": js }),
            timeout_ms: 10_000,
        }),
        CliCommand::Wx { method, args } => Ok(RpcCall {
            method: "wx",
            params: json!({
                "method": method,
                "args": args.iter().map(|arg| parse_wx_arg(arg)).collect::<Vec<_>>(),
            }),
            timeout_ms: 10_000,
        }),
        CliCommand::NetStart {
            port,
            https,
            timeout_ms,
        } => Ok(RpcCall {
            method: "net.start",
            params: json!({ "port": port, "https": https }),
            timeout_ms: *timeout_ms,
        }),
        CliCommand::NetStop { timeout_ms } => Ok(RpcCall {
            method: "net.stop",
            params: json!({}),
            timeout_ms: *timeout_ms,
        }),
        CliCommand::NetLog {
            filter,
            limit,
            with_body,
            ..
        } => Ok(RpcCall {
            method: "net.log",
            params: json!({ "filter": filter, "limit": limit, "withBody": with_body }),
            timeout_ms: 10_000,
        }),
        CliCommand::NetMock {
            url,
            json_file,
            method,
        } => Ok(RpcCall {
            method: "net.mock",
            params: json!({
                "url": url,
                "jsonFile": json_file,
                "method": normalize_http_method(method),
            }),
            timeout_ms: 10_000,
        }),
        CliCommand::NetUnmock { url } => Ok(RpcCall {
            method: "net.unmock",
            params: json!({ "url": url }),
            timeout_ms: 10_000,
        }),
        CliCommand::NetClear => Ok(simple_call("net.clear")),
        CliCommand::NetInstallCa => Ok(simple_call("net.install-ca")),
        CliCommand::DaemonStop => Ok(RpcCall {
            method: "daemon.stop",
            params: json!({}),
            timeout_ms: 5_000,
        }),
        CliCommand::DaemonRestart => Ok(simple_call("status")),
    }
}

pub fn execute(command: CliCommand) -> Result<Value, CliError> {
    match command {
        CliCommand::Start {
            proxy,
            proxy_https,
            project_path,
            cli_path,
            appid,
            auto_port,
            timeout_ms,
        } => {
            let start = prepared_command(CliCommand::Start {
                project_path,
                cli_path,
                appid,
                auto_port,
                proxy,
                proxy_https,
                timeout_ms,
            })?;
            if proxy {
                let _ = execute_rpc(CliCommand::NetStart {
                    port: 8899,
                    https: proxy_https,
                    timeout_ms: 30_000,
                })?;
            }
            execute_rpc(start)
        }
        CliCommand::DaemonStop => {
            let result = run_stop_daemon()?;
            Ok(json!({
                "ok": true,
                "daemonKind": DAEMON_KIND,
                "forced": result.forced,
            }))
        }
        CliCommand::DaemonRestart => {
            let _ = run_stop_daemon();
            std::thread::sleep(Duration::from_millis(1_500));
            let response = execute_rpc(prepared_command(CliCommand::Status)?)?;
            Ok(inject_daemon_kind("status", response))
        }
        other => {
            let is_status = matches!(other, CliCommand::Status);
            let response = execute_rpc(prepared_command(other)?)?;
            if is_status {
                Ok(inject_daemon_kind("status", response))
            } else {
                Ok(response)
            }
        }
    }
}

fn prepared_command(command: CliCommand) -> Result<CliCommand, CliError> {
    match command {
        CliCommand::Start {
            project_path,
            cli_path,
            appid,
            auto_port,
            proxy,
            proxy_https,
            timeout_ms,
        } => {
            let project_path = match project_path {
                Some(project_path) => Some(project_path),
                None => Some(resolve_project_path()?),
            };
            Ok(CliCommand::Start {
                project_path,
                cli_path,
                appid,
                auto_port,
                proxy,
                proxy_https,
                timeout_ms,
            })
        }
        other => Ok(other),
    }
}

fn execute_rpc(command: CliCommand) -> Result<Value, CliError> {
    let call = build_rpc_call(&command)?;
    let probe = ensure_daemon_available()?;
    let response = call_with_timeout(
        &probe.socket_path,
        call.method,
        call.params,
        call.timeout_ms,
    )
    .map_err(map_daemon_error)?;

    if let Some(error) = response.error {
        if error.code == ERROR_REF_EXPIRED {
            if let Some(reference) = error
                .data
                .as_ref()
                .and_then(|value| value.get("ref"))
                .and_then(Value::as_u64)
            {
                return Err(CliError::RefExpired(reference));
            }
        }
        return Err(CliError::Message(error.message));
    }

    response
        .result
        .ok_or_else(|| CliError::Message("missing rpc result".into()))
}

fn ensure_daemon_available() -> Result<ProbeResult, CliError> {
    let deps = ConnectDeps {
        connect: Box::new(|socket_path| {
            Box::pin(async move {
                UnixStream::connect(&socket_path)
                    .map(|stream| {
                        drop(stream);
                        ProbeResult {
                            socket_path,
                            daemon_kind: String::from(DAEMON_KIND),
                        }
                    })
                    .map_err(|error| error.to_string())
            })
        }),
        spawn_daemon: Box::new(spawn_daemon_process),
        wait: Box::new(|ms| {
            Box::pin(async move {
                std::thread::sleep(Duration::from_millis(ms));
            })
        }),
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| CliError::Message(error.to_string()))?
        .block_on(connect_to_daemon(deps))
        .map_err(map_daemon_error)
}

fn run_stop_daemon() -> Result<crate::daemon_client::StopResult, CliError> {
    let runtime = runtime_dir();
    let socket_path = runtime.join(RUST_SOCKET_FILE);
    let pid_path = runtime.join(RUST_PID_FILE);
    let pid_from_file = read_pid_file(&pid_path);
    let listed_pids = list_process_ids();
    let deps = StopDeps {
        rpc_stop: Box::new(move || {
            let socket_path = socket_path.clone();
            Box::pin(async move {
                call_with_timeout(&socket_path, "daemon.stop", json!({}), 5_000)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            })
        }),
        probe_running: Box::new(move || {
            let socket_path = runtime_dir().join(RUST_SOCKET_FILE);
            Box::pin(async move { UnixStream::connect(socket_path).is_ok() })
        }),
        pid_from_file,
        listed_pids,
        kill_pid: Box::new(kill_pid),
        remove_runtime_file: Box::new(remove_runtime_file),
        wait: Box::new(|ms| {
            Box::pin(async move {
                std::thread::sleep(Duration::from_millis(ms));
            })
        }),
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| CliError::Message(error.to_string()))?
        .block_on(stop_daemon(deps))
        .map_err(map_daemon_error)
}

fn inject_daemon_kind(method: &str, response: Value) -> Value {
    if method != "status" {
        return response;
    }
    match response {
        Value::Object(mut object) => {
            if !object.contains_key("daemonKind") {
                object.insert(
                    String::from("daemonKind"),
                    Value::String(String::from(DAEMON_KIND)),
                );
            }
            Value::Object(object)
        }
        other => other,
    }
}

fn resolve_project_path() -> Result<String, CliError> {
    let root = env::current_dir().map_err(|error| CliError::Message(error.to_string()))?;
    let selector = Selector::new(prompt_for_project_selection);
    detect_or_select_project(&root, io::stdin().is_terminal(), selector)
        .map(|path| path.display().to_string())
        .map_err(|error| CliError::Message(error.to_string()))
}

fn prompt_for_project_selection(
    candidates: &[PathBuf],
) -> Result<usize, crate::project_detect::DetectProjectError> {
    println!("找到多个候选项目：");
    for (index, candidate) in candidates.iter().enumerate() {
        println!("  {}. {}", index + 1, candidate.display());
    }
    print!("请选择 (输入序号): ");
    io::stdout().flush().map_err(|error| {
        crate::project_detect::DetectProjectError::SelectionFailed {
            message: error.to_string(),
        }
    })?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).map_err(|error| {
        crate::project_detect::DetectProjectError::SelectionFailed {
            message: error.to_string(),
        }
    })?;
    let index = answer.trim().parse::<usize>().map_err(|error| {
        crate::project_detect::DetectProjectError::SelectionFailed {
            message: error.to_string(),
        }
    })?;
    index.checked_sub(1).ok_or_else(
        || crate::project_detect::DetectProjectError::SelectionFailed {
            message: String::from("selector returned invalid index: 0"),
        },
    )
}

fn parse_ref(reference: &str) -> Result<u32, CliError> {
    reference
        .strip_prefix('%')
        .ok_or_else(|| CliError::Message("ref must be in %N format".into()))?
        .parse::<u32>()
        .map_err(|_| CliError::Message("ref must be in %N format".into()))
}

fn parse_wait_ref(reference: &str) -> Result<Value, CliError> {
    match reference.strip_prefix('%') {
        Some(_) => parse_ref(reference).map(Value::from),
        None => Ok(Value::String(reference.to_string())),
    }
}

fn parse_wx_arg(arg: &str) -> Value {
    serde_json::from_str(arg).unwrap_or_else(|_| Value::String(arg.to_string()))
}

fn optional_object<const N: usize>(entries: [(&str, Option<Value>); N]) -> Value {
    let mut map = Map::new();
    for (key, value) in entries {
        if let Some(value) = value {
            map.insert(String::from(key), value);
        }
    }
    Value::Object(map)
}

fn normalize_http_method(method: &str) -> String {
    let trimmed = method.trim();
    if trimmed.is_empty() {
        String::from("GET")
    } else {
        trimmed.to_ascii_uppercase()
    }
}

fn simple_call(method: &'static str) -> RpcCall {
    RpcCall {
        method,
        params: json!({}),
        timeout_ms: 10_000,
    }
}

fn runtime_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wxpilot")
}

fn spawn_daemon_process() -> Result<(), String> {
    let daemon_bin = locate_daemon_binary();
    let mut command = Command::new(daemon_bin);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn locate_daemon_binary() -> PathBuf {
    let current_exe = env::current_exe().ok();
    locate_daemon_binary_from(current_exe.as_deref())
}

fn locate_daemon_binary_from(current_exe: Option<&Path>) -> PathBuf {
    if let Some(path) = env::var_os("WXP_DAEMON_BIN") {
        return PathBuf::from(path);
    }

    if let Some(current_exe) = current_exe {
        if let Some(directory) = current_exe.parent() {
            let sibling = directory.join("wxp-daemon");
            if sibling.exists() {
                return sibling;
            }
        }
    }

    let private_install = runtime_dir().join("bin").join("wxp-daemon");
    if private_install.exists() {
        return private_install;
    }

    PathBuf::from("wxp-daemon")
}

fn read_pid_file(path: &Path) -> Option<u32> {
    let raw = fs::read_to_string(path).ok()?;
    raw.trim().parse::<u32>().ok()
}

fn list_process_ids() -> Vec<u32> {
    let pattern = locate_daemon_binary();
    let output = Command::new("pgrep")
        .args(["-f", &pattern.to_string_lossy()])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let current_pid = std::process::id();
    let mut pids = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Ok(pid) = line.trim().parse::<u32>() {
            if pid > 0 && pid != current_pid {
                pids.insert(pid);
            }
        }
    }
    pids.into_iter().collect()
}

fn kill_pid(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        return if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error().to_string())
        };
    }

    #[cfg(not(unix))]
    Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .map_err(|error| error.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("failed to kill pid {pid}"))
            }
        })
}

fn remove_runtime_file(path: PathBuf) {
    if let Err(error) = fs::remove_file(path) {
        if error.kind() != io::ErrorKind::NotFound {
            let _ = error;
        }
    }
}

fn map_daemon_error(message: String) -> CliError {
    if let Some(stripped) = message.strip_prefix("daemon_unavailable: ") {
        CliError::DaemonUnavailable(stripped.to_string())
    } else {
        CliError::Message(message)
    }
}

#[cfg(test)]
mod tests {
    use super::locate_daemon_binary_from;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn set_home(path: &Path) {
        // SAFETY: tests hold a process-wide mutex around env mutation.
        unsafe {
            env::set_var("HOME", path);
        }
    }

    fn clear_daemon_bin() {
        // SAFETY: tests hold a process-wide mutex around env mutation.
        unsafe {
            env::remove_var("WXP_DAEMON_BIN");
        }
    }

    fn set_daemon_bin(path: &Path) {
        // SAFETY: tests hold a process-wide mutex around env mutation.
        unsafe {
            env::set_var("WXP_DAEMON_BIN", path);
        }
    }

    #[test]
    fn locate_daemon_binary_prefers_explicit_env_override() {
        let _guard = env_lock().lock().expect("lock env");
        let home = tempdir().expect("home");
        set_home(home.path());
        clear_daemon_bin();

        let sibling_dir = tempdir().expect("sibling");
        let sibling = sibling_dir.path().join("wxp-daemon");
        fs::write(&sibling, "#!/bin/sh\n").expect("write sibling");

        let private_dir = home.path().join(".wxpilot/bin");
        fs::create_dir_all(&private_dir).expect("mkdir private");
        fs::write(private_dir.join("wxp-daemon"), "#!/bin/sh\n").expect("write private");

        let custom = home.path().join("custom/wxp-daemon");
        set_daemon_bin(&custom);

        assert_eq!(
            locate_daemon_binary_from(Some(&sibling_dir.path().join("wxpilot"))),
            custom
        );

        clear_daemon_bin();
    }

    #[test]
    fn locate_daemon_binary_prefers_sibling_before_private_install() {
        let _guard = env_lock().lock().expect("lock env");
        let home = tempdir().expect("home");
        set_home(home.path());
        clear_daemon_bin();

        let sibling_dir = tempdir().expect("sibling");
        let sibling = sibling_dir.path().join("wxp-daemon");
        fs::write(&sibling, "#!/bin/sh\n").expect("write sibling");

        let private_dir = home.path().join(".wxpilot/bin");
        fs::create_dir_all(&private_dir).expect("mkdir private");
        fs::write(private_dir.join("wxp-daemon"), "#!/bin/sh\n").expect("write private");

        assert_eq!(
            locate_daemon_binary_from(Some(&sibling_dir.path().join("wxpilot"))),
            sibling
        );
    }

    #[test]
    fn locate_daemon_binary_uses_private_install_when_present() {
        let _guard = env_lock().lock().expect("lock env");
        let home = tempdir().expect("home");
        set_home(home.path());
        clear_daemon_bin();

        let private_daemon = home.path().join(".wxpilot/bin/wxp-daemon");
        fs::create_dir_all(private_daemon.parent().expect("parent")).expect("mkdir private");
        fs::write(&private_daemon, "#!/bin/sh\n").expect("write private");

        assert_eq!(locate_daemon_binary_from(None), private_daemon);
    }

    #[test]
    fn locate_daemon_binary_falls_back_to_path_when_no_other_candidate_exists() {
        let _guard = env_lock().lock().expect("lock env");
        let home = tempdir().expect("home");
        set_home(home.path());
        clear_daemon_bin();

        assert_eq!(locate_daemon_binary_from(None), PathBuf::from("wxp-daemon"));
    }
}
