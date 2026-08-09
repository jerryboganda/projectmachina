//! Spawns the shared Node loopback fixture server
//! (`scripts/test/fixture-server-cli.mjs`, itself a thin CLI wrapper around
//! `scripts/test/fixture-server.mjs`) as a subprocess, per the task's
//! instruction to extend and reuse the existing fixture infrastructure
//! rather than build parallel test infra. Requires `node` on `PATH`.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

#[derive(Debug, serde::Deserialize)]
pub struct FixtureAddress {
    pub host: String,
    pub port: u16,
    pub protocol: String,
}

#[derive(serde::Deserialize)]
struct FixtureAnnouncement {
    instances: Vec<FixtureAddress>,
}

pub struct FixtureProcess {
    child: Child,
    pub instances: Vec<FixtureAddress>,
}

fn repo_root() -> PathBuf {
    // crates/network -> crates -> repo root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/network has two ancestor directories up to the repo root")
        .to_path_buf()
}

impl FixtureProcess {
    /// Spawn `instances` independent fixture-server listeners (each its own
    /// loopback origin -- different ports are different origins) in a
    /// single Node process.
    pub fn spawn(instances: u32) -> Self {
        let root = repo_root();
        let mut child = Command::new("node")
            .arg("scripts/test/fixture-server-cli.mjs")
            .arg(format!("--instances={instances}"))
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node must be on PATH to run the shared loopback fixture server");

        let stdout = child.stdout.take().expect("piped stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .expect("fixture server prints one announcement line before serving");
        child.stdout = Some(reader.into_inner());

        let announcement: FixtureAnnouncement = serde_json::from_str(line.trim())
            .unwrap_or_else(|error| panic!("invalid fixture announcement {line:?}: {error}"));

        Self {
            child,
            instances: announcement.instances,
        }
    }

    pub fn origin(&self, index: usize) -> String {
        let address = &self.instances[index];
        format!("{}://{}:{}", address.protocol, address.host, address.port)
    }
}

impl Drop for FixtureProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
