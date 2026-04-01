use std::{
    fmt, fs, io,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunAutoRequest {
    pub project_path: String,
    pub port: u16,
    pub explicit: Option<PathBuf>,
    pub appid: Option<String>,
}

type RunAutoHook = Arc<dyn Fn(&RunAutoRequest) -> io::Result<String> + Send + Sync>;

#[derive(Clone)]
pub struct DevtoolsLauncher {
    candidates: Vec<PathBuf>,
    run_auto_timeout: Duration,
    run_auto_hook: Option<RunAutoHook>,
}

impl DevtoolsLauncher {
    const DEFAULT_RUN_AUTO_TIMEOUT: Duration = Duration::from_secs(30);
    const POLL_INTERVAL: Duration = Duration::from_millis(100);

    pub fn new(candidates: Vec<PathBuf>) -> Self {
        Self {
            candidates,
            run_auto_timeout: Self::DEFAULT_RUN_AUTO_TIMEOUT,
            run_auto_hook: None,
        }
    }

    pub fn with_run_auto_timeout(mut self, timeout: Duration) -> Self {
        self.run_auto_timeout = timeout;
        self
    }

    pub fn with_run_auto_hook<F>(mut self, hook: F) -> Self
    where
        F: Fn(&RunAutoRequest) -> io::Result<String> + Send + Sync + 'static,
    {
        self.run_auto_hook = Some(Arc::new(hook));
        self
    }

    pub fn default_candidates() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/Applications/wechatwebdevtools.app/Contents/MacOS/cli"),
            PathBuf::from("/Applications/Wechat Devtools Stable.app/Contents/MacOS/cli"),
        ]
    }

    pub fn find_cli(&self, explicit: Option<PathBuf>) -> io::Result<PathBuf> {
        if let Some(path) = explicit {
            return Self::validate_cli_path(&path);
        }

        for candidate in &self.candidates {
            if let Ok(path) = Self::validate_candidate_path(candidate) {
                return Ok(path);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "wechat_devtools_cli_not_found",
        ))
    }

    pub fn run_auto(
        &self,
        project_path: &str,
        port: u16,
        explicit: Option<PathBuf>,
        appid: Option<&str>,
    ) -> io::Result<String> {
        let request = RunAutoRequest {
            project_path: project_path.to_string(),
            port,
            explicit,
            appid: appid.map(str::to_string),
        };

        if let Some(hook) = &self.run_auto_hook {
            return hook(&request);
        }

        let cli = self.find_cli(request.explicit.clone())?;
        let mut command = Command::new(cli);
        command.args([
            "auto",
            "--project",
            &request.project_path,
            "--auto-port",
            &request.port.to_string(),
        ]);

        if let Some(appid) = request.appid.as_deref() {
            command.args(["--appid", appid]);
        }

        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");
        let stdout_reader = thread::spawn(move || read_pipe(stdout));
        let stderr_reader = thread::spawn(move || read_pipe(stderr));
        let status = self.wait_for_exit(&mut child)?;
        let stdout = stdout_reader
            .join()
            .map_err(|_| io::Error::other("cli stdout reader panicked"))??;
        let stderr = stderr_reader
            .join()
            .map_err(|_| io::Error::other("cli stderr reader panicked"))??;
        let merged_output = format!(
            "{}{}",
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr),
        );

        if status.success() {
            return Ok(format!("ws://127.0.0.1:{}", request.port));
        }

        Err(io::Error::other(format!(
            "cli auto failed: {merged_output}"
        )))
    }

    fn wait_for_exit(&self, child: &mut Child) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + self.run_auto_timeout;

        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "cli auto timed out",
                ));
            }

            thread::sleep(Self::POLL_INTERVAL);
        }
    }

    fn validate_candidate_path(path: &Path) -> io::Result<PathBuf> {
        match fs::metadata(path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "wechat_devtools_cli_is_not_file",
                    ));
                }
                Self::validate_executable(path)
            }
            Err(error) => {
                if error.kind() == io::ErrorKind::NotFound {
                    Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "wechat_devtools_cli_not_found",
                    ))
                } else {
                    Err(error)
                }
            }
        }
    }

    fn validate_cli_path(path: &Path) -> io::Result<PathBuf> {
        match fs::metadata(path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "wechat_devtools_cli_is_not_file",
                    ));
                }
                Self::validate_executable(path)
            }
            Err(error) => {
                if error.kind() == io::ErrorKind::NotFound {
                    Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "wechat_devtools_cli_not_found",
                    ))
                } else {
                    Err(error)
                }
            }
        }
    }

    #[cfg(unix)]
    fn validate_executable(path: &Path) -> io::Result<PathBuf> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path)?;
        let mode = metadata.permissions().mode();
        if mode & 0o111 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "wechat_devtools_cli_not_executable",
            ));
        }
        Ok(path.to_path_buf())
    }

    #[cfg(not(unix))]
    fn validate_executable(path: &Path) -> io::Result<PathBuf> {
        Ok(path.to_path_buf())
    }
}

impl fmt::Debug for DevtoolsLauncher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DevtoolsLauncher")
            .field("candidates", &self.candidates)
            .field("run_auto_timeout", &self.run_auto_timeout)
            .field("has_run_auto_hook", &self.run_auto_hook.is_some())
            .finish()
    }
}

fn read_pipe<R: Read>(mut reader: R) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok(output)
}
