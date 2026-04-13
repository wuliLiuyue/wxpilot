use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use wxp_cli::daemon_client::{connect_to_daemon, stop_daemon, ConnectDeps, ProbeResult, StopDeps};

#[tokio::test]
async fn only_spawns_rust_daemon_when_probe_fails() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_connect = attempts.clone();
    let deps = ConnectDeps {
        connect: Box::new(move |_| {
            let current = attempts_for_connect.fetch_add(1, Ordering::SeqCst) + 1;
            let result = if current < 3 {
                Err("ECONNREFUSED".into())
            } else {
                Ok(ProbeResult {
                    socket_path: "/tmp/rust.sock".into(),
                    daemon_kind: "rust".into(),
                })
            };
            Box::pin(async move { result })
        }),
        spawn_daemon: Box::new(|| Ok(())),
        wait: Box::new(|_| Box::pin(async {})),
    };

    let result = connect_to_daemon(deps).await.expect("connect");
    assert_eq!(result.daemon_kind, "rust");
}

#[tokio::test]
async fn does_not_spawn_daemon_when_initial_probe_succeeds() {
    let spawn_attempts = Arc::new(AtomicUsize::new(0));
    let spawn_attempts_for_daemon = spawn_attempts.clone();
    let deps = ConnectDeps {
        connect: Box::new(|_| {
            Box::pin(async {
                Ok(ProbeResult {
                    socket_path: "/tmp/rust.sock".into(),
                    daemon_kind: "rust".into(),
                })
            })
        }),
        spawn_daemon: Box::new(move || {
            spawn_attempts_for_daemon.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
        wait: Box::new(|_| Box::pin(async {})),
    };

    let result = connect_to_daemon(deps).await.expect("connect");
    assert_eq!(result.daemon_kind, "rust");
    assert_eq!(spawn_attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn returns_daemon_unavailable_after_retry_budget_is_exhausted() {
    let spawn_attempts = Arc::new(AtomicUsize::new(0));
    let spawn_attempts_for_daemon = spawn_attempts.clone();
    let deps = ConnectDeps {
        connect: Box::new(|_| Box::pin(async { Err("ECONNREFUSED".into()) })),
        spawn_daemon: Box::new(move || {
            spawn_attempts_for_daemon.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
        wait: Box::new(|_| Box::pin(async {})),
    };

    let error = match connect_to_daemon(deps).await {
        Ok(_) => panic!("connect should fail"),
        Err(error) => error,
    };
    assert_eq!(
        error,
        "daemon_unavailable: failed to connect to rust daemon"
    );
    assert_eq!(spawn_attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn forced_stop_kills_known_pids_when_rpc_stop_fails() {
    let removed = Arc::new(Mutex::new(Vec::new()));
    let removed_for_files = removed.clone();
    let waits = Arc::new(AtomicUsize::new(0));
    let waits_for_stop = waits.clone();
    let deps = StopDeps {
        rpc_stop: Box::new(|| Box::pin(async { Err("timeout after 5000ms".into()) })),
        probe_running: Box::new(|| Box::pin(async { false })),
        pid_from_file: None,
        listed_pids: vec![1234, 5678],
        kill_pid: Box::new(|_| Ok(())),
        remove_runtime_file: Box::new(move |path| {
            removed_for_files
                .lock()
                .expect("lock removed files")
                .push(path);
        }),
        wait: Box::new(move |_| {
            waits_for_stop.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {})
        }),
    };

    let result = stop_daemon(deps).await.expect("stop");
    assert!(result.forced);
    assert_eq!(result.killed_pids, vec![1234, 5678]);
    let removed = removed.lock().expect("lock removed files");
    assert_eq!(removed.len(), 2);
    assert!(removed
        .iter()
        .any(|path| path.ends_with("rust-daemon.sock")));
    assert!(removed.iter().any(|path| path.ends_with("rust-daemon.pid")));
    assert_eq!(waits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn forced_stop_still_kills_pid_from_file_when_rpc_stop_succeeds_and_probe_reports_running() {
    let removed = Arc::new(Mutex::new(Vec::new()));
    let removed_for_files = removed.clone();
    let deps = StopDeps {
        rpc_stop: Box::new(|| Box::pin(async { Ok(()) })),
        probe_running: Box::new(|| Box::pin(async { true })),
        pid_from_file: Some(4321),
        listed_pids: vec![1234, 5678],
        kill_pid: Box::new(|_| Ok(())),
        remove_runtime_file: Box::new(move |path| {
            removed_for_files
                .lock()
                .expect("lock removed files")
                .push(path);
        }),
        wait: Box::new(|_| Box::pin(async {})),
    };

    let result = stop_daemon(deps).await.expect("stop");
    assert!(result.forced);
    assert_eq!(result.killed_pids, vec![4321]);
    assert!(removed.lock().expect("lock removed files").is_empty());
}

#[tokio::test]
async fn forced_stop_still_kills_pid_from_file_when_probe_reports_running() {
    let removed = Arc::new(Mutex::new(Vec::new()));
    let removed_for_files = removed.clone();
    let deps = StopDeps {
        rpc_stop: Box::new(|| Box::pin(async { Err("timeout after 5000ms".into()) })),
        probe_running: Box::new(|| Box::pin(async { true })),
        pid_from_file: Some(4321),
        listed_pids: vec![1234, 5678],
        kill_pid: Box::new(|_| Ok(())),
        remove_runtime_file: Box::new(move |path| {
            removed_for_files
                .lock()
                .expect("lock removed files")
                .push(path);
        }),
        wait: Box::new(|_| Box::pin(async {})),
    };

    let result = stop_daemon(deps).await.expect("stop");
    assert!(result.forced);
    assert_eq!(result.killed_pids, vec![4321]);
    assert!(removed.lock().expect("lock removed files").is_empty());
}

#[tokio::test]
async fn forced_stop_does_not_report_pid_when_kill_fails() {
    let removed = Arc::new(Mutex::new(Vec::new()));
    let removed_for_files = removed.clone();
    let deps = StopDeps {
        rpc_stop: Box::new(|| Box::pin(async { Err("timeout after 5000ms".into()) })),
        probe_running: Box::new(|| Box::pin(async { false })),
        pid_from_file: Some(4321),
        listed_pids: vec![1234, 5678],
        kill_pid: Box::new(|_| Err("operation not permitted".into())),
        remove_runtime_file: Box::new(move |path| {
            removed_for_files
                .lock()
                .expect("lock removed files")
                .push(path);
        }),
        wait: Box::new(|_| Box::pin(async {})),
    };

    let result = stop_daemon(deps).await.expect("stop");
    assert!(!result.forced);
    assert!(result.killed_pids.is_empty());
    let removed = removed.lock().expect("lock removed files");
    assert_eq!(removed.len(), 2);
    assert!(removed
        .iter()
        .any(|path| path.ends_with("rust-daemon.sock")));
    assert!(removed.iter().any(|path| path.ends_with("rust-daemon.pid")));
}
