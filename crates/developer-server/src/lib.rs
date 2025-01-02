use std::process::{Command, Child};

/// Starts the developer server as a subprocess
pub fn start_developer_server() -> Child {
    Command::new("cargo")
        .args(&["run", "-p", "developer-server"])
        .spawn()
        .expect("Failed to start developer server")
}

/// Stops the developer server subprocess
pub fn stop_developer_server(child: &mut Child) {
    child.kill().expect("Failed to stop developer server");
    child.wait().expect("Failed to wait for developer server");
}
