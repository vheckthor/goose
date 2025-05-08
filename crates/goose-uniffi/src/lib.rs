// Tell UniFFI to generate glue for everything annotated below:
uniffi::setup_scaffolding!();

pub mod types;
mod tool_result_serde;

use crate::types::Message;

#[uniffi::export]
pub fn print_messages(messages: Vec<Message>) {
    for msg in messages {
        println!(
            "[{:?} @ {}] {:?}",
            msg.role,
            msg.created,
            msg.content
        );
    }
}

