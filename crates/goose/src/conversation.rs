use crate::message::Message;
use crate::message::MessageContent::{self};
use crate::token_counter::TokenCounter;
use mcp_core::Role;
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub enum InteractionType {
    QnA,
    BeginToolUse,
    InsideToolUse,
    OutsideToolUse,
    Stub,
}

#[derive(Clone, Debug)]
pub struct Interaction {
    pub query: Option<Message>,
    pub reply: Option<Message>,
    pub token_count: usize,
    pub kind: Option<InteractionType>,
    pub linked: Vec<Interaction>,
}

impl Default for Interaction {
    fn default() -> Self {
        Interaction {
            query: None,
            reply: None,
            token_count: 0,
            kind: None,
            linked: Vec::new(),
        }
    }
}

impl Interaction {
    pub fn new() -> Self {
        Interaction::default()
    }

    pub fn record(&mut self, message: &Message, token_counter: &TokenCounter) {
        match (&self.query, &self.reply) {
            (None, None) => {
                self.query = Some(message.clone());
                self.token_count += self.text_content_size(Some(message), token_counter);
                self.kind = Some(InteractionType::Stub);
            }
            (Some(_), None) => {
                self.reply = Some(message.clone());
                self.token_count += self.text_content_size(Some(message), token_counter);
                self.kind = self.classify_interaction_type();
            }
            _ => ()
        }
    }

    pub fn classify_interaction_type(&self) -> Option<InteractionType> {
        let (query, reply) = match (&self.query, &self.reply) {
            (Some(q), Some(r)) => (q, r),
            _ => return None,
        };

        if query.role != Role::User || reply.role != Role::Assistant {
            return None;
        }

        let is_tool_response = query.content.iter().any(|c| matches!(c, MessageContent::ToolResponse(_)));
        let is_tool_request = reply.content.iter().any(|c| matches!(c, MessageContent::ToolRequest(_)));

        match (is_tool_response, is_tool_request) {
            (false, false) => Some(InteractionType::QnA),
            (false, true) => Some(InteractionType::BeginToolUse),
            (true, true) => Some(InteractionType::InsideToolUse),
            (true, false) => Some(InteractionType::OutsideToolUse),
        }
    }

    fn text_content_size(&self, message: Option<&Message>, token_counter: &TokenCounter) -> usize {
        message.map_or(0, |msg| {
            msg.content.iter()
                .filter_map(|content| {
                    let plain_text = content.as_text();
                    let tool_text = content.as_tool_response_text();

                    match (plain_text, tool_text) {
                        (Some(plain), Some(tool)) => {
                            // Compare lengths and return the longer text
                            if plain.len() > tool.len() {
                                Some(plain.to_string())
                            } else {
                                Some(tool)
                            }
                        }
                        (Some(plain), None) => Some(plain.to_string()),
                        (None, Some(tool)) => Some(tool),
                        (None, None) => None,
                    }
                })
                .map(|text| token_counter.count_tokens(&text))
                .sum()
        })
    }

    pub fn add_linked_interaction(&mut self, interaction: Interaction) {
        self.token_count += &interaction.token_count;
        self.linked.push(interaction);
    }
}

#[derive(Debug)]
pub struct ConversationError(String);

impl std::fmt::Display for ConversationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ConversationError: {}", self.0)
    }
}

impl std::error::Error for ConversationError {}

#[derive(Clone, Debug)]
pub struct Conversation {
    pub interactions: Vec<Interaction>,
}

impl Default for Conversation {
    fn default() -> Self {
        Conversation { interactions: vec![] }
    }
}

impl Conversation {
    pub fn new(interactions: Vec<Interaction>) -> Self {
        Conversation { interactions }
    }

    pub fn parse(messages: &[Message], token_counter: &TokenCounter) -> Result<Self, Box<dyn std::error::Error>> {
        if messages.is_empty() {
            return Err(Box::new(ConversationError("Conversation cannot be empty".to_string())));
        }

        if messages[0].role != Role::User {
            return Err(Box::new(ConversationError("First message must be from User".to_string())));
        }

        let mut remaining_msgs: VecDeque<Message> = VecDeque::from(messages.to_vec());
        let mut interactions: Vec<Interaction> = Vec::new();
        let mut current_interaction_head: Option<usize> = None;
        let mut interaction = Interaction::new();

        while let Some(msg) = remaining_msgs.pop_front() {
            if let Some(last_msg) = interactions.last().and_then(|i| i.reply.as_ref()) {
                if let Some(query) = &interaction.query {
                    if query.role == last_msg.role {
                        return Err(Box::new(ConversationError("Speaker roles must alternate".to_string())));
                    }
                }
            }

            interaction.record(&msg, token_counter);

            if let Some(kind) = &interaction.kind {
                match kind {
                    InteractionType::QnA
                    | InteractionType::BeginToolUse => {
                        interactions.push(interaction.clone());
                        current_interaction_head = Some(interactions.len() - 1);
                        interaction = Interaction::new();
                    }
                    InteractionType::Stub => {
                        if remaining_msgs.is_empty() {
                            if let Some(head_idx) = current_interaction_head {
                                interactions[head_idx].add_linked_interaction(interaction.clone());
                            }
                        }
                    }
                    _ => {
                        if let Some(head_idx) = current_interaction_head {
                            interactions[head_idx].add_linked_interaction(interaction.clone());
                        } else {
                            return Err(Box::new(ConversationError("First interaction must be QnA or BeginToolUse".to_string())));
                        }
                        interaction = Interaction::new();
                    }
                }
            }
        }

        Ok(Conversation::new(interactions))
    }

    pub fn render(&self) -> Vec<Message> {
        let mut messages = Vec::new();
        let mut last_role = None;

        for interaction in &self.interactions {
            // Add main interaction messages
            if let Some(query) = &interaction.query {
                if last_role != Some(&query.role) {
                    messages.push(query.clone());
                    last_role = Some(&query.role);
                }
            }
            if let Some(reply) = &interaction.reply {
                if last_role != Some(&reply.role) {
                    messages.push(reply.clone());
                    last_role = Some(&reply.role);
                }
            }

            // Add linked interaction messages
            for linked in &interaction.linked {
                if let Some(query) = &linked.query {
                    if last_role != Some(&query.role) {
                        messages.push(query.clone());
                        last_role = Some(&query.role);
                    }
                }
                if let Some(reply) = &linked.reply {
                    if last_role != Some(&reply.role) {
                        messages.push(reply.clone());
                        last_role = Some(&reply.role);
                    }
                }
            }
        }

        messages
    }
}

#[cfg(test)]
mod tests {
    use mcp_core::{Content, TextContent, ToolCall};
    use crate::providers::configs::GPT_4O_TOKENIZER;
    use super::*;

    fn create_text_message(role: Role, text: &str) -> Message {
        match role {
            Role::User => Message::user().with_text(text),
            Role::Assistant => Message::assistant().with_text(text),
        }
    }

    fn create_tool_response_message(text: &str) -> Message {
        let content = vec![Content::Text(TextContent {
            text: text.to_string(),
            annotations: None,
        })];
        let result = Ok(content);
        Message::user().with_tool_response("test_id", result)
    }

    fn create_tool_request_message(name: &str, args: &str) -> Message {
        let tool_call = Ok(ToolCall {
            name: name.to_string(),
            arguments: args.to_string().parse().unwrap(),
        });
        Message::assistant().with_tool_request("test_id", tool_call)
    }

    #[test]
    fn test_conversation_must_start_with_user_message() {
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        let messages = vec![
            create_text_message(Role::Assistant, "Hello"),
            create_text_message(Role::User, "Hi"),
        ];

        let result = Conversation::parse(&messages, &token_counter);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("First message must be from User"));
    }

    #[test]
    fn test_conversation_must_start_with_qna_interaction() {
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        let messages = vec![
            create_tool_response_message("tool response"),
            create_tool_request_message("test_tool", "{}"),
        ];

        let result = Conversation::parse(&messages, &token_counter);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("First interaction must be QnA"));
    }

    #[test]
    fn test_rendered_messages_alternate_roles() {
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        let messages = vec![
            create_text_message(Role::User, "Hello"),
            create_text_message(Role::Assistant, "Hi"),
            create_text_message(Role::User, "How are you?"),
            create_text_message(Role::Assistant, "I'm good"),
        ];

        let conversation = Conversation::parse(&messages, &token_counter).unwrap();
        let rendered = conversation.render();

        for i in 0..rendered.len() {
            if i % 2 == 0 {
                assert_eq!(rendered[i].role, Role::User);
            } else {
                assert_eq!(rendered[i].role, Role::Assistant);
            }
        }
    }

    #[test]
    fn test_linked_interactions_maintain_role_alternation() {
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        let messages = vec![
            create_text_message(Role::User, "Hello"),
            create_text_message(Role::Assistant, "Let me check something"),
            create_tool_response_message("tool response"), //user
            create_tool_request_message("test_tool", "{}"), // agent
            create_text_message(Role::User, "Thanks"),
            create_text_message(Role::Assistant, "You're welcome"),
        ];

        let conversation = Conversation::parse(&messages, &token_counter).unwrap();
        let rendered = conversation.render();

        for i in 0..rendered.len() {
            if i % 2 == 0 {
                assert_eq!(rendered[i].role, Role::User);
            } else {
                assert_eq!(rendered[i].role, Role::Assistant);
            }
        }
    }

    #[test]
    fn test_empty_conversation_is_rejected() {
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        let result = Conversation::parse(&[], &token_counter);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Conversation cannot be empty"));
    }
}