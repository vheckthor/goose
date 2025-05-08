// Tell UniFFI to generate glue for everything annotated below:
uniffi::setup_scaffolding!();

mod tool_result_serde;
pub mod types;

use crate::types::Message;

#[uniffi::export]
pub fn print_messages(messages: Vec<Message>) {
    for msg in messages {
        println!("[{:?} @ {}] {:?}", msg.role, msg.created, msg.content);
    }
}
