//! Real [`ProcessRunner`] backed by `tokio::process`. No shell involved — matches
//! [`emu_core::ports::Command`]'s own contract.

use std::process::Stdio;

use async_trait::async_trait;
use emu_core::error::CoreError;
use emu_core::ports::{ChildProcess, Command as PortCommand, Output as PortOutput, ProcessRunner};
use emu_core::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

/// Spawns real OS processes.
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeProcessRunner;

fn build_command(cmd: &PortCommand) -> Command {
    let mut command = Command::new(&cmd.program);
    command.args(&cmd.args);
    if let Some(cwd) = &cmd.cwd {
        command.current_dir(cwd);
    }
    for (key, value) in &cmd.env {
        command.env(key, value);
    }
    // `Stdio::null()` when there's nothing to feed: an unconditionally-piped stdin that we never
    // close would leave a child that reads until EOF (e.g. `cat`, or `sdkmanager` without
    // `--licenses` input) hanging forever waiting for a write end we're not going to provide.
    let stdin = if cmd.stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    };
    command
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

async fn feed_stdin(program: &str, child: &mut Child, stdin: Option<&Vec<u8>>) -> Result<()> {
    let Some(bytes) = stdin else { return Ok(()) };
    let mut pipe = child.stdin.take().ok_or_else(|| CoreError::Process {
        program: program.to_string(),
        code: -1,
        stderr: "stdin pipe unavailable".to_string(),
    })?;
    pipe.write_all(bytes)
        .await
        .map_err(|e| process_err(program, &e))?;
    // Dropping `pipe` here closes the write end, sending EOF — required for tools like
    // `sdkmanager --licenses` that read prompts from stdin until it closes.
    Ok(())
}

fn process_err(program: &str, e: &std::io::Error) -> CoreError {
    CoreError::Process {
        program: program.to_string(),
        code: -1,
        stderr: e.to_string(),
    }
}

#[async_trait]
impl ProcessRunner for NativeProcessRunner {
    async fn run(&self, cmd: PortCommand) -> Result<PortOutput> {
        let mut child = build_command(&cmd)
            .spawn()
            .map_err(|e| process_err(&cmd.program, &e))?;
        feed_stdin(&cmd.program, &mut child, cmd.stdin.as_ref()).await?;
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| process_err(&cmd.program, &e))?;
        Ok(PortOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    async fn spawn(&self, cmd: PortCommand) -> Result<Box<dyn ChildProcess>> {
        let mut child = build_command(&cmd)
            .spawn()
            .map_err(|e| process_err(&cmd.program, &e))?;
        feed_stdin(&cmd.program, &mut child, cmd.stdin.as_ref()).await?;

        let (tx, rx) = mpsc::unbounded_channel();
        if let Some(stdout) = child.stdout.take() {
            spawn_line_forwarder(stdout, tx.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_line_forwarder(stderr, tx.clone());
        }
        drop(tx); // the forwarders hold their own clones; drop ours so the channel can close.

        Ok(Box::new(NativeChild {
            program: cmd.program,
            child,
            lines: rx,
        }))
    }
}

fn spawn_line_forwarder<R>(reader: R, tx: mpsc::UnboundedSender<String>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
}

/// A live child process, streaming merged stdout/stderr lines.
///
/// Simplification (recorded, not accidental): unlike [`NativeProcessRunner::run`], `wait()` here
/// cannot tell which stream a buffered line came from once lines are merged, so any lines left
/// undrained by [`ChildProcess::next_line`] are folded into `Output::stdout`; `stderr` is always
/// empty. Every consumer so far (log tailing during `emulator`/`sdkmanager` launches) only needs
/// the merged text, not the split.
struct NativeChild {
    program: String,
    child: Child,
    lines: mpsc::UnboundedReceiver<String>,
}

#[async_trait]
impl ChildProcess for NativeChild {
    fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    async fn next_line(&mut self) -> Result<Option<String>> {
        Ok(self.lines.recv().await)
    }

    async fn wait(&mut self) -> Result<PortOutput> {
        let mut leftover = String::new();
        while let Some(line) = self.lines.recv().await {
            leftover.push_str(&line);
            leftover.push('\n');
        }
        let status = self
            .child
            .wait()
            .await
            .map_err(|e| process_err(&self.program, &e))?;
        Ok(PortOutput {
            status: status.code().unwrap_or(-1),
            stdout: leftover,
            stderr: String::new(),
        })
    }

    async fn kill(&mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .map_err(|e| process_err(&self.program, &e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emu_core::ports::Command as PortCommand;

    #[tokio::test]
    async fn run_captures_stdout_stderr_and_exit_code() {
        let runner = NativeProcessRunner;
        let out = runner
            .run(
                PortCommand::new("sh")
                    .arg("-c")
                    .arg("echo out; echo err >&2; exit 3"),
            )
            .await
            .expect("run");
        assert_eq!(out.status, 3);
        assert!(!out.success());
        assert_eq!(out.stdout.trim(), "out");
        assert_eq!(out.stderr.trim(), "err");
    }

    #[tokio::test]
    async fn run_feeds_stdin_and_closes_it() {
        let runner = NativeProcessRunner;
        let mut cmd = PortCommand::new("cat");
        cmd.stdin = Some(b"piped input".to_vec());
        let out = runner.run(cmd).await.expect("run");
        assert_eq!(out.stdout, "piped input");
        assert!(out.success());
    }

    #[tokio::test]
    async fn spawn_streams_merged_lines_then_waits() {
        let runner = NativeProcessRunner;
        let mut child = runner
            .spawn(PortCommand::new("sh").arg("-c").arg("echo one; echo two"))
            .await
            .expect("spawn");
        assert!(child.pid().is_some());

        let mut got = Vec::new();
        while let Some(line) = child.next_line().await.expect("next_line") {
            got.push(line);
        }
        got.sort();
        assert_eq!(got, vec!["one".to_string(), "two".to_string()]);

        let out = child.wait().await.expect("wait");
        assert!(out.success());
    }

    #[tokio::test]
    async fn kill_stops_a_long_running_child() {
        let runner = NativeProcessRunner;
        let mut child = runner
            .spawn(PortCommand::new("sleep").arg("30"))
            .await
            .expect("spawn");
        child.kill().await.expect("kill");
        let out = child.wait().await.expect("wait after kill");
        assert!(!out.success());
    }
}
