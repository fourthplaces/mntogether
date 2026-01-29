//! Command builder pattern for running external processes

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// Fluent builder for running external commands
#[derive(Default)]
pub struct CmdBuilder {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    envs: Vec<(String, String)>,
    inherit_io: bool,
    capture_stdout: bool,
    capture_stderr: bool,
}

impl CmdBuilder {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            ..Default::default()
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    pub fn inherit_io(mut self) -> Self {
        self.inherit_io = true;
        self
    }

    pub fn capture_stdout(mut self) -> Self {
        self.capture_stdout = true;
        self
    }

    pub fn capture_stderr(mut self) -> Self {
        self.capture_stderr = true;
        self
    }

    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        cmd
    }

    pub fn run(&self) -> Result<i32> {
        let mut cmd = self.build_command();
        if self.inherit_io {
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
        }
        let status = cmd.status().with_context(|| {
            format!("failed to start: {} {}", self.program, self.args.join(" "))
        })?;
        Ok(status.code().unwrap_or(1))
    }

    pub fn run_capture(&self) -> Result<CmdOutput> {
        let mut cmd = self.build_command();
        // Explicitly set stdin to null to prevent hanging on interactive prompts
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output: Output = cmd
            .spawn()
            .with_context(|| format!("failed to start: {} {}", self.program, self.args.join(" ")))?
            .wait_with_output()
            .with_context(|| {
                format!(
                    "failed to wait for: {} {}",
                    self.program,
                    self.args.join(" ")
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "{} exited with code {:?}: {}",
                self.program,
                output.status.code(),
                stderr.trim()
            ));
        }
        let mut stdout = output.stdout;
        if self.capture_stderr {
            stdout.extend_from_slice(&output.stderr);
        }
        Ok(CmdOutput {
            stdout,
            code: output.status.code().unwrap_or(0),
        })
    }
}

/// Output from a captured command execution
pub struct CmdOutput {
    pub stdout: Vec<u8>,
    #[allow(dead_code)]
    pub code: i32,
}

impl CmdOutput {
    pub fn stdout_string(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }

    pub fn stdout_lines(&self) -> Vec<String> {
        self.stdout_string()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }
}
