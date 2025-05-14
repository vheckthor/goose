use crate::errors::BenchResult;
use crate::logging;
use std::env;
use std::process::{Child, Command};
use std::thread::JoinHandle;

pub fn await_process_exits(
    child_processes: &mut [Child],
    handles: Vec<JoinHandle<BenchResult<()>>>,
) {
    for child in child_processes.iter_mut() {
        match child.wait() {
            Ok(status) => logging::info(&format!("Child exited with status: {}", status)),
            Err(e) => logging::error(&format!("Error waiting for child: {}", e)),
        }
    }

    for handle in handles {
        match handle.join() {
            Ok(_res) => (),
            Err(e) => {
                // Handle thread panic
                logging::error(&format!("Thread panicked: {:?}", e));
            }
        }
    }
}

pub fn parallel_bench_cmd(bench_cmd: String, config: String, envs: Vec<(String, String)>) -> Child {
    let current_exe = env::current_exe().expect("Failed to get current executable path");

    let mut cmd = Command::new(current_exe);
    cmd.arg("bench").arg(bench_cmd).arg("--config").arg(config);

    for (key, value) in envs.into_iter() {
        cmd.env(key, value);
    }

    cmd.spawn().expect("Failed to spawn child process")
}
