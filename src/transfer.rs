use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use ssh2::Session;
use std::fs::File;
use std::io::Read;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;
use crate::config::Destination;

const BUFFER_SIZE: usize = 262_144;

pub struct TransferStats {
    pub bytes_transferred: u64,
    pub duration_secs: f64,
}

pub struct Transferer {
    destination: Destination,
    verbose: bool,
}

impl Transferer {
    pub fn new(destination: Destination, verbose: bool) -> Self {
        Self { destination, verbose }
    }

    pub fn transfer(&self, local_path: &str, ssh_key_path: &str) -> Result<TransferStats> {
        let start_time = Instant::now();
        let path = PathBuf::from(local_path);
        
        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", local_path);
        }

        let session = self.connect(ssh_key_path)?;
        let sftp = session.sftp()
            .context("Failed to initialize SFTP")?;

        let mut total_bytes = 0u64;

        if path.is_file() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {msg}")
                    .unwrap()
            );
            pb.set_message(format!("Uploading {}", path.file_name().unwrap().to_string_lossy()));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            let remote_file_path = PathBuf::from(&self.destination.remote_path)
                .join(path.file_name().unwrap());
            total_bytes = self.upload_file(&sftp, &path, remote_file_path.to_str().unwrap())?;
            
            pb.finish_with_message(format!("✓ Uploaded {}", path.file_name().unwrap().to_string_lossy()));
        } else {
            let files: Vec<_> = WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .collect();

            let total_files = files.len();
            
            let pb = ProgressBar::new(total_files as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files {msg}")
                    .unwrap()
                    .progress_chars("#>-")
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            for entry in files {
                let file_path = entry.path();
                let relative = file_path.strip_prefix(&path)
                    .context("Failed to compute relative path")?;
                
                let remote_file_path = PathBuf::from(&self.destination.remote_path)
                    .join(path.file_name().unwrap())
                    .join(relative);

                pb.set_message(format!("Uploading {}", relative.display()));
                
                total_bytes += self.upload_file(&sftp, file_path, remote_file_path.to_str().unwrap())?;
                pb.inc(1);
            }

            pb.finish_with_message(format!("✓ Uploaded {} files", total_files));
        }

        let duration = start_time.elapsed();
        Ok(TransferStats {
            bytes_transferred: total_bytes,
            duration_secs: duration.as_secs_f64(),
        })
    }

    fn connect(&self, ssh_key_path: &str) -> Result<Session> {
        if self.verbose {
            eprintln!("Connecting to {}:{}", self.destination.host, self.destination.port);
        }
        let tcp = TcpStream::connect(format!("{}:{}", self.destination.host, self.destination.port))
            .context("Failed to connect to server")?;

        tcp.set_nodelay(true)
            .context("Failed to set TCP_NODELAY")?;
        
        use std::os::unix::io::AsRawFd;
        let fd = tcp.as_raw_fd();
        unsafe {
            let size: libc::c_int = 2_097_152;
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                &size as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &size as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
        }

        if self.verbose {
            eprintln!("Creating SSH session");
        }
        let mut session = Session::new()
            .context("Failed to create SSH session")?;
        
        session.set_tcp_stream(tcp);
        if self.verbose {
            eprintln!("Performing SSH handshake");
        }
        session.handshake()
            .context("SSH handshake failed")?;

        if let Some(ref password) = self.destination.password {
            if self.verbose {
                eprintln!("Authenticating with password for user: {}", self.destination.username);
            }
            session.userauth_password(&self.destination.username, password)
                .context("Password authentication failed")?;
        } else {
            if self.verbose {
                eprintln!("Authenticating with SSH key: {} for user: {}", ssh_key_path, self.destination.username);
            }
            session.userauth_pubkey_file(
                &self.destination.username,
                None,
                Path::new(ssh_key_path),
                None,
            ).context("SSH key authentication failed")?;
        }

        if !session.authenticated() {
            anyhow::bail!("Authentication failed");
        }
        
        if self.verbose {
            eprintln!("Successfully authenticated");
        }
        Ok(session)
    }

    fn upload_file(&self, sftp: &ssh2::Sftp, local_path: &Path, remote_path: &str) -> Result<u64> {
        if self.verbose {
            eprintln!("Uploading: {} -> {}", local_path.display(), remote_path);
        }
        
        let remote_dir = Path::new(remote_path).parent()
            .context("Invalid remote path")?;
        
        if self.verbose {
            eprintln!("Ensuring remote directory exists: {}", remote_dir.display());
        }
        self.ensure_remote_dir(sftp, remote_dir)?;

        if self.verbose {
            eprintln!("Opening local file: {}", local_path.display());
        }
        let mut local_file = File::open(local_path)
            .context("Failed to open local file")?;
        
        if self.verbose {
            eprintln!("Creating remote file: {}", remote_path);
        }
        let mut remote_file = sftp.create(Path::new(remote_path))
            .context(format!("Failed to create remote file: {}", remote_path))?;

        let mut buffer = vec![0; BUFFER_SIZE];
        let mut total_bytes = 0u64;
        loop {
            let bytes_read = local_file.read(&mut buffer)
                .context("Failed to read local file")?;
            
            if bytes_read == 0 {
                break;
            }

            std::io::Write::write_all(&mut remote_file, &buffer[..bytes_read])
                .context("Failed to write to remote file")?;
            total_bytes += bytes_read as u64;
        }

        Ok(total_bytes)
    }

    fn ensure_remote_dir(&self, sftp: &ssh2::Sftp, dir: &Path) -> Result<()> {
        if self.verbose {
            eprintln!("Checking if directory exists: {}", dir.display());
        }
        if sftp.stat(dir).is_ok() {
            if self.verbose {
                eprintln!("Directory already exists: {}", dir.display());
            }
            return Ok(());
        }

        if let Some(parent) = dir.parent() {
            if self.verbose {
                eprintln!("Creating parent directory first: {}", parent.display());
            }
            self.ensure_remote_dir(sftp, parent)?;
        }

        if self.verbose {
            eprintln!("Creating directory: {}", dir.display());
        }
        sftp.mkdir(dir, 0o755)
            .context(format!("Failed to create remote directory: {}", dir.display()))?;
        if self.verbose {
            eprintln!("Successfully created directory: {}", dir.display());
        }

        Ok(())
    }
}
