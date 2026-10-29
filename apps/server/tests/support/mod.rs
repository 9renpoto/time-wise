use reqwest::{blocking::Client, header::HeaderMap, Method};
use serde_json::Value;
use std::{
    fs,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

pub fn run(command: &mut Command, timeout: Duration) -> Result<String, String> {
    let mut output = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut diagnostics = tempfile::tempfile().map_err(|e| e.to_string())?;
    command.stdout(Stdio::from(output.try_clone().map_err(|e| e.to_string())?));
    command.stderr(Stdio::from(
        diagnostics.try_clone().map_err(|e| e.to_string())?,
    ));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if Instant::now() >= deadline {
            #[cfg(unix)]
            let _ = Command::new("kill")
                .args(["-KILL", &format!("-{}", child.id())])
                .status();
            let _ = child.kill();
            let _ = child.wait();
            return Err("subprocess timed out".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    use std::io::{Seek, SeekFrom};
    output.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut text = String::new();
    output
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if !status.success() {
        diagnostics
            .seek(SeekFrom::Start(0))
            .map_err(|e| e.to_string())?;
        let mut errors = String::new();
        diagnostics
            .read_to_string(&mut errors)
            .map_err(|e| e.to_string())?;
        return Err(format!("subprocess failed ({status}): {text}{errors}"));
    }
    Ok(text.trim().to_owned())
}

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn build(script: &str) {
    // Coverage belongs to this native harness, not the separately executed Wasm.
    run(
        Command::new("sh")
            .arg(root().join(script))
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env_remove("LLVM_PROFILE_FILE")
            .env("CARGO_TARGET_DIR", root().join("target/smoke-build")),
        Duration::from_secs(600),
    )
    .unwrap();
}

pub struct Sandbox {
    pub project: TempDir,
    name: String,
    client: Client,
    active: bool,
}

impl Sandbox {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Self {
            project: tempfile::tempdir().unwrap(),
            name: format!(
                "rust-smoke-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ),
            client: Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap(),
            active: false,
        }
    }

    pub fn copy(&self, source: &Path, destination: &str) {
        let target = self.project.path().join(destination);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source, target).unwrap();
    }

    pub fn configure(&self, config: &Value) {
        fs::write(
            self.project.path().join("wrangler.jsonc"),
            config.to_string(),
        )
        .unwrap();
    }

    fn compose(&self, args: &[&str]) -> Result<String, String> {
        let mut command = Command::new("docker");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let owner = fs::metadata(self.project.path()).map_err(|error| error.to_string())?;
            command.env("CELLD_UID", owner.uid().to_string());
            command.env("CELLD_GID", owner.gid().to_string());
        }
        run(
            command
                .args(["compose", "-f"])
                .arg(root().join("compose.celld.yaml"))
                .args(["-p", &self.name])
                .args(args)
                .env("CELLD_PROJECT_DIR", self.project.path())
                .env("CELLD_PORT", "0"),
            Duration::from_secs(120),
        )
    }

    pub fn start(&mut self) -> String {
        self.active = true;
        self.compose(&["up", "-d"]).unwrap();
        let address: SocketAddr = self
            .compose(&["port", "celld", "9876"])
            .unwrap()
            .parse()
            .unwrap();
        assert!(address.ip().is_loopback());
        format!("http://{address}")
    }

    pub fn stop(&mut self) {
        self.compose(&["down"]).unwrap();
        self.active = false;
    }

    pub fn request(
        &self,
        base: &str,
        path: &str,
        method: Method,
    ) -> Result<(u16, Value, HeaderMap), String> {
        let response = self
            .client
            .request(method, format!("{base}{path}"))
            .send()
            .map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response.json().map_err(|e| e.to_string())?;
        Ok((status, body, headers))
    }

    pub fn wait(&self, base: &str, path: &str, expected: impl Fn(u16, &Value) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if let Ok((status, body, _)) = self.request(base, path, Method::GET) {
                if expected(status, &body) {
                    return;
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
        panic!("celld did not reach the expected response in 30 seconds");
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if self.active {
            if thread::panicking() {
                if let Ok(logs) = self.compose(&["logs", "--tail", "40"]) {
                    eprintln!("{logs}");
                }
            }
            if let Err(error) = self.compose(&["down"]) {
                eprintln!("cleanup failed: {error}");
            }
        }
    }
}
