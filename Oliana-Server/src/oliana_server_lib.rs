#![allow(unused_imports, unused_variables)]

use futures::prelude::*;
use tarpc::{
    client, context,
    server::{self, Channel},
};

// This is the service definition. It looks a lot like a trait definition.
// It defines one RPC, hello, which takes one arg, name, and returns a String.
#[tarpc::service]
pub trait Oliana {
    /// Runs an LLM and returns immediately; callers should concatinate results of generate_text_next_token() until it returns None for the reply. Return is some diagnostic text from server.
    async fn generate_text_begin(prompt: String) -> String;
    /// Returns None when token generation is complete
    async fn generate_text_next_token() -> Option<String>;
}

// This is the type that implements the generated World trait. It is the business logic
// and is used to start the server.
// There will be one OlianaServer client for each TCP connection; a dis-connect and re-connect will allocate a new OlianaServer.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct OlianaServer {
    pub client_socket: std::net::SocketAddr,

    #[serde(skip)]
    pub shareable_procs: Option<std::sync::Arc<std::sync::RwLock<oliana_lib::launchers::TrackedProcs>>>,

    pub token_generation_complete: bool,
    pub generated_text_tokens: Vec<String>,
    pub generate_text_next_token_i: usize,
}

impl OlianaServer {
    pub fn new(client_socket: std::net::SocketAddr, shareable_procs: std::sync::Arc<std::sync::RwLock<oliana_lib::launchers::TrackedProcs>>) -> Self {
        Self {
            client_socket: client_socket,

            shareable_procs: Some(shareable_procs),

            token_generation_complete: false,
            generated_text_tokens: Vec::with_capacity(4096),
            generate_text_next_token_i: 0,

        }
    }
}

// These methods are run in the context of the client connection, on the server.
impl Oliana for OlianaServer {
    async fn generate_text_begin(mut self, _: context::Context, prompt: String) -> String {

        self.token_generation_complete = false;
        self.generate_text_next_token_i = 0;

        format!("Hello, {prompt}!")

    }

    async fn generate_text_next_token(mut self, _: context::Context) -> Option<String> {
        // We poll until either self.token_generation_complete or the vec has enough tokens to return the next one
        while !self.token_generation_complete && self.generate_text_next_token_i < self.generated_text_tokens.len()-1 {
            tokio::time::sleep( tokio::time::Duration::from_millis(200) ).await;
        }

        let token = self.generated_text_tokens.get(self.generate_text_next_token_i);
        if token.is_some() {
            self.generate_text_next_token_i += 1;
        }
        return token.cloned();
    }

}




