//! PTY-backed interactive shell (proper TUI/resize support).

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use tracing::info;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Run `shell` attached to a pseudo-terminal, relaying I/O until exit or Ctrl+C.
pub fn run_interactive_shell(shell: &str) -> Result<()> {
    SHUTDOWN.store(false, Ordering::SeqCst);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("open pty")?;

    let mut cmd = CommandBuilder::new(shell);
    cmd.env(
        "TERM",
        std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()),
    );

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .with_context(|| format!("spawn shell in pty: {}", shell))?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().context("clone pty reader")?;
    let mut writer = pair.master.take_writer().context("pty writer")?;

    let writer_thread = thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 8192];
        loop {
            if SHUTDOWN.load(Ordering::SeqCst) {
                break;
            }
            match stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if writer.write_all(&buf[..n]).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
                Err(_) => break,
            }
        }
    });

    let mut stdout = std::io::stdout();
    let mut buf = [0u8; 8192];
    loop {
        if SHUTDOWN.load(Ordering::SeqCst) {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if stdout.write_all(&buf[..n]).is_err() {
                    break;
                }
                let _ = stdout.flush();
            }
            Err(_) => break,
        }
    }

    SHUTDOWN.store(true, Ordering::SeqCst);
    let _ = writer_thread.join();

    match child.try_wait() {
        Ok(Some(status)) => {
            info!("shell exited: {:?}", status);
        }
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
        }
        Err(e) => {
            tracing::warn!("wait on shell failed: {}", e);
        }
    }

    Ok(())
}

/// Signal the PTY relay loop to stop (used on Ctrl+C).
pub fn request_shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
}
