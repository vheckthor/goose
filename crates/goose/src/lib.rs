pub mod agents;
pub mod developer;
pub mod errors;
pub mod key_manager;
pub mod memory;
pub mod message;
pub mod process_store;
pub mod prompt_template;
pub mod providers;
pub mod systems;
pub mod token_counter;

use developer_server::start_developer_server;

pub fn run_goose_with_developer_server() {
    let mut developer_server = start_developer_server();

    println!("Running Goose application...");
    // Simulate Goose work
    // std::thread::sleep(std::time::Duration::from_secs(10));

    // println!("Shutting down Goose application...");
    // stop_developer_server(&mut developer_server);
}
