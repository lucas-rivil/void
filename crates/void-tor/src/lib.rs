pub mod control;

pub use control::ControlClient;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{sleep, Instant};

#[derive(Debug, Error)]
pub enum TorError {
    #[error("tor.exe introuvable dans {0}. Lancez scripts/fetch-tor.ps1")]
    MissingTor(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("protocole control: {0}")]
    Control(String),
    #[error("délai dépassé en attendant: {0}")]
    Timeout(String),
    #[error("le processus tor s'est arrêté prématurément{0} — un antivirus l'a peut-être bloqué (ajoutez une exclusion pour tor.exe)")]
    ProcessDied(String),
}

pub type TorResult<T> = Result<T, TorError>;

pub struct TorConfig {
    pub tor_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl TorConfig {
    pub fn tor_exe(&self) -> PathBuf {
        self.tor_dir.join("tor.exe")
    }

    pub fn port_file(&self) -> PathBuf {
        self.data_dir.join("control.port")
    }

    pub fn cookie_file(&self) -> PathBuf {
        self.data_dir.join("control_auth_cookie")
    }

    pub fn torrc_path(&self) -> PathBuf {
        self.data_dir.join("void-torrc")
    }
}

pub struct TorBoot {
    child: Child,
    pub control: ControlClient,
    _job: Option<JobGuard>,
}

impl TorBoot {
    pub fn is_alive(&mut self) -> bool {
        self.child
            .try_wait()
            .map(|state| state.is_none())
            .unwrap_or(false)
    }

    pub fn exit_status(&mut self) -> String {
        match self.child.try_wait() {
            Ok(Some(status)) => match status.code() {
                Some(code) => format!(" (code {code})"),
                None => " (terminé par un signal)".to_string(),
            },
            _ => String::new(),
        }
    }
}

pub struct TorHandle {
    child: Child,
    control: ControlClient,
    socks: SocketAddr,
    onion_id: String,
    _job: Option<JobGuard>,
}

#[cfg(windows)]
pub(crate) struct JobGuard(winapi::shared::ntdef::HANDLE);

#[cfg(windows)]
unsafe impl Send for JobGuard {}

#[cfg(windows)]
unsafe impl Sync for JobGuard {}

#[cfg(not(windows))]
pub(crate) struct JobGuard;

#[cfg(windows)]
impl Drop for JobGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                winapi::um::handleapi::CloseHandle(self.0);
            }
        }
    }
}

#[cfg(windows)]
fn assign_kill_on_close_job(child_pid: u32) -> Option<JobGuard> {
    use std::mem;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::jobapi2::{
        AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    };
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winnt::{
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JobObjectExtendedLimitInformation, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };
    unsafe {
        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, child_pid);
        if process.is_null() {
            return None;
        }
        let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
        if job.is_null() {
            CloseHandle(process);
            return None;
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &mut info as *mut _ as *mut _,
            mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if configured == 0 {
            CloseHandle(process);
            CloseHandle(job);
            return None;
        }
        let assigned = AssignProcessToJobObject(job, process);
        CloseHandle(process);
        if assigned == 0 {
            CloseHandle(job);
            return None;
        }
        Some(JobGuard(job))
    }
}

impl TorHandle {
    pub fn socks(&self) -> SocketAddr {
        self.socks
    }

    pub fn onion_id(&self) -> &str {
        &self.onion_id
    }

    pub fn is_alive(&mut self) -> bool {
        self.child
            .try_wait()
            .map(|state| state.is_none())
            .unwrap_or(false)
    }

    pub async fn shutdown(mut self) -> TorResult<()> {
        let _ = self.control.send("SIGNAL SHUTDOWN").await;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }
        let _ = self.child.kill().await;
        Ok(())
    }
}

impl TorBoot {
    pub async fn socks_addr(&mut self) -> TorResult<SocketAddr> {
        let raw = self.control.get_info("net/listeners/socks").await?;
        let addr = raw.trim().trim_matches('"');
        addr.parse::<SocketAddr>()
            .map_err(|e| TorError::Control(format!("adresse socks invalide ({addr}): {e}")))
    }

    pub async fn bootstrap_progress(&mut self) -> TorResult<(u8, String)> {
        let raw = self.control.get_info("status/bootstrap-phase").await?;
        let progress = extract_kv(&raw, "PROGRESS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let tag = extract_kv(&raw, "TAG").unwrap_or_default();
        Ok((progress, tag))
    }

    pub async fn circuit_established(&mut self) -> TorResult<bool> {
        let raw = self
            .control
            .get_info("status/circuit-established")
            .await?;
        Ok(raw.trim() == "1")
    }

    pub async fn publish_onion(
        self,
        key_b64: &str,
        virtual_port: u16,
        target_port: u16,
    ) -> TorResult<TorHandle> {
        let mut boot = self;
        let onion_id = boot
            .control
            .add_onion(key_b64, virtual_port, target_port)
            .await?;
        let socks = boot.socks_addr().await?;
        Ok(TorHandle {
            child: boot.child,
            control: boot.control,
            socks,
            onion_id,
            _job: boot._job,
        })
    }
}

fn extract_kv(s: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    s.split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix).map(|v| v.to_string()))
}

pub async fn launch(cfg: &TorConfig) -> TorResult<TorBoot> {
    if !cfg.tor_exe().exists() {
        return Err(TorError::MissingTor(cfg.tor_dir.clone()));
    }
    std::fs::create_dir_all(&cfg.data_dir)?;
    let _ = std::fs::remove_file(cfg.port_file());

    let torrc = build_torrc(cfg);
    std::fs::write(cfg.torrc_path(), &torrc)?;

    let mut child = Command::new(cfg.tor_exe())
        .arg("-f")
        .arg(cfg.torrc_path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true)
        .spawn()?;

    #[cfg(windows)]
    let job = child.id().and_then(assign_kill_on_close_job);
    #[cfg(not(windows))]
    let job = None;

    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(pipe_logs(BufReader::new(stdout), "tor"));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(pipe_logs(BufReader::new(stderr), "tor!"));
    }

    let control_addr = wait_for_control_addr(cfg).await?;
    let control = ControlClient::connect_authenticated(control_addr, &cfg.cookie_file()).await?;
    Ok(TorBoot {
        child,
        control,
        _job: job,
    })
}

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

async fn pipe_logs<R>(mut reader: BufReader<R>, target: &'static str)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => tracing::debug!(target: "void::tor", "[{target}] {}", line.trim_end()),
        }
    }
}

fn slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn build_torrc(cfg: &TorConfig) -> String {
    format!(
        "SocksPort 127.0.0.1:auto OnionTrafficOnly\n\
         ControlPort 127.0.0.1:auto\n\
         ControlPortWriteToFile \"{}\"\n\
         CookieAuthentication 1\n\
         ClientOnly 1\n\
         DataDirectory \"{}\"\n\
         GeoIPFile \"{}\"\n\
         GeoIPv6File \"{}\"\n\
         Log notice stdout\n\
         AvoidDiskWrites 1\n",
        slash(&cfg.port_file()),
        slash(&cfg.data_dir),
        slash(&cfg.tor_dir.join("geoip")),
        slash(&cfg.tor_dir.join("geoip6")),
    )
}

async fn wait_for_control_addr(cfg: &TorConfig) -> TorResult<SocketAddr> {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if let Ok(content) = std::fs::read_to_string(cfg.port_file()) {
            if let Some(addr) = content.trim().strip_prefix("PORT=") {
                if let Ok(addr) = addr.trim().parse::<SocketAddr>() {
                    return Ok(addr);
                }
            }
        }
        sleep(Duration::from_millis(250)).await;
    }
    Err(TorError::Timeout("ControlPort disponible".into()))
}
