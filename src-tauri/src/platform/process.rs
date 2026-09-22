//! Bounded subprocess execution; arguments never pass through a shell.
use crate::error::{AppError, AppResult};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn run(program: &str, args: &[&str]) -> AppResult<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::AutomationUnavailable(format!("{program}: {e}")))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = stdout {
            let _ = pipe.take(1_048_576).read_to_end(&mut bytes);
        }
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = stderr {
            let _ = pipe.take(65536).read_to_end(&mut bytes);
        }
        bytes
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            other => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(AppError::AutomationUnavailable(format!(
                    "{program} failed or timed out: {other:?}"
                )));
            }
        }
    };
    let output = out.join().unwrap_or_default();
    let error = err.join().unwrap_or_default();
    if !status?.success() {
        return Err(AppError::AutomationUnavailable(format!(
            "{program}: {}",
            String::from_utf8_lossy(&error).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output).trim_end().to_string())
}
