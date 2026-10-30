//! SOPS CLI boundary. Document and plaintext travel through pipes, never temp files.
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

fn executable() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut locations: Vec<_> = std::env::split_paths(&path).collect();
    if cfg!(target_os = "macos") {
        locations.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ]);
    }
    let name = if cfg!(windows) { "sops.exe" } else { "sops" };
    locations
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| name.into())
}

pub(crate) fn transform(
    input: &str,
    key: &str,
    format: &str,
    decrypt: bool,
) -> Result<String, String> {
    let workspace = super::workspace::Workspace::temporary("devkits-sops")?;
    // Empty configuration prevents encryption from picking up unrelated project rules.
    let config = workspace.0.join("config.yaml");
    std::fs::write(&config, "creation_rules:\n  - {}\n")
        .map_err(|_| "Could not prepare SOPS configuration.")?;
    let empty_keys = workspace.0.join("empty-keys.txt");
    std::fs::write(&empty_keys, "").map_err(|_| "Could not prepare isolated SOPS key sources.")?;
    let mut command = Command::new(executable());
    command.env_clear();
    for name in ["PATH", "SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .current_dir(&workspace.0)
        .env("XDG_CONFIG_HOME", &workspace.0)
        .env("SOPS_AGE_KEY_FILE", &empty_keys)
        .env("SOPS_AGE_SSH_PRIVATE_KEY_FILE", &empty_keys)
        .env("AWS_EC2_METADATA_DISABLED", "true")
        .args(["--config"])
        .arg(&config)
        .arg(if decrypt { "decrypt" } else { "encrypt" })
        .args(["--input-type", format, "--output-type", format]);
    if decrypt {
        command
            .env("SOPS_AGE_KEY", key)
            .args(["--decryption-order", "age"]);
    } else {
        command.args([
            "--age",
            key,
            "--filename-override",
            &format!("document.{format}"),
        ]);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound {
        "SOPS CLI is required (3.13+). Install sops and add it to PATH; macOS: brew install sops.".to_string()
    } else { "Could not start SOPS. Check that the executable is accessible.".to_string() })?;
    let mut stdin = child.stdin.take().ok_or("Could not open SOPS stdin.")?;
    let stdout = child.stdout.take().ok_or("Could not open SOPS stdout.")?;
    let stderr = child.stderr.take().ok_or("Could not open SOPS stderr.")?;
    // Concurrent pipe draining avoids deadlocks on inputs/results larger than a pipe buffer.
    std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(input.as_bytes()));
        let reader = scope.spawn(|| {
            let mut bytes = Zeroizing::new(Vec::new());
            stdout
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        });
        let errors = scope.spawn(|| {
            // Discard diagnostics: malformed input may cause SOPS to echo secret values.
            let mut sink = std::io::sink();
            std::io::copy(&mut stderr.take(64 * 1024), &mut sink)
        });
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if started.elapsed() < Duration::from_secs(30) => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err("SOPS did not complete within 30 seconds.".to_string());
                }
            }
        };
        let written = writer.join().map_err(|_| "SOPS input task failed.")?;
        let bytes = reader
            .join()
            .map_err(|_| "SOPS output task failed.")?
            .map_err(|_| "Could not read SOPS result.")?;
        let _ = errors.join();
        let status = status?;
        if !status.success() {
            return Err(if decrypt {
                "SOPS decryption failed. Check the format, supply the complete document including sops metadata, and use its matching age identity. Modified data/MAC or unavailable additional key groups also cause failure."
            } else {
                "SOPS encryption failed. Check the selected document format and age recipient. SOPS 3.13+ is required."
            }.into());
        }
        written.map_err(|_| "Could not pass the complete document to SOPS.")?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("SOPS result exceeds 8 MiB.".into());
        }
        std::str::from_utf8(&bytes)
            .map(str::to_owned)
            .map_err(|_| "SOPS result is not UTF-8.".into())
    })
}
