#![allow(unused_imports, unused_variables, unused_mut)]

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
    async fn generate_text_begin(system_prompt: String, user_prompt: String) -> String;
    /// Returns None when token generation is complete
    async fn generate_text_next_token() -> Option<String>;
}

// This is the type that implements the generated World trait. It is the business logic
// and is used to start the server.
// There will be one OlianaServer client for each TCP connection; a dis-connect and re-connect will allocate a new OlianaServer.
// Also for each message OlianaServer::clone() is called -_- necessitaging syncronization primitives
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct OlianaServer {
    pub client_socket: std::net::SocketAddr,

    #[serde(skip)]
    pub shareable_procs: Option<std::sync::Arc<std::sync::RwLock<oliana_lib::launchers::TrackedProcs>>>,

    #[serde(skip)]
    pub ai_workdir_images: String,
    #[serde(skip)]
    pub ai_workdir_text: String,

    pub text_input_nonce: std::sync::Arc<std::sync::RwLock<usize>>,
    pub token_generation_complete: std::sync::Arc<std::sync::RwLock<bool>>,
    pub generated_text_tokens: std::sync::Arc<std::sync::RwLock<Vec<String>>>,
    pub generate_text_next_token_i: std::sync::Arc<std::sync::RwLock<usize>>,
}

impl OlianaServer {
    pub fn new(client_socket: std::net::SocketAddr,
               shareable_procs: std::sync::Arc<std::sync::RwLock<oliana_lib::launchers::TrackedProcs>>,
               ai_workdir_images: &str,
               ai_workdir_text: &str
        ) -> Self {
        Self {
            client_socket: client_socket,

            shareable_procs: Some(shareable_procs),
            ai_workdir_images: ai_workdir_images.to_string(),
            ai_workdir_text: ai_workdir_text.to_string(),

            text_input_nonce: std::sync::Arc::new(std::sync::RwLock::new( 0 )),

            token_generation_complete: std::sync::Arc::new(std::sync::RwLock::new( false )),
            generated_text_tokens: std::sync::Arc::new(std::sync::RwLock::new( Vec::with_capacity(4096) )),
            generate_text_next_token_i: std::sync::Arc::new(std::sync::RwLock::new( 0 )),

        }
    }

    pub fn read_text_input_nonce(&self) -> usize {
        let mut ret_val: usize = 0;
        match self.text_input_nonce.read() {
            Ok(text_input_nonce_rg) => {
                ret_val = *text_input_nonce_rg;
            }
            Err(e) => {
                eprintln!("{}:{} {:?}", file!(), line!(), e);
            }
        }
        ret_val
    }

    pub async fn increment_to_next_free_text_input_nonce(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        while tokio::fs::try_exists( self.get_current_text_input_json_path() ).await? {
            if let Ok(ref mut text_input_nonce_wg) = self.text_input_nonce.write() {
                **text_input_nonce_wg += 1;
            }
        }
        Ok(self.read_text_input_nonce())
    }
    pub fn get_current_text_input_json_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.ai_workdir_text).join(format!("{}.json", self.read_text_input_nonce()))
    }
    pub fn get_current_text_output_txt_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.ai_workdir_text).join(format!("{}.txt", self.read_text_input_nonce()))
    }

    pub fn read_generate_text_next_token_i(&self) -> usize {
        let mut ret_val: usize = 0;
        match self.generate_text_next_token_i.read() {
            Ok(generate_text_next_token_i_rg) => {
                ret_val = *generate_text_next_token_i_rg;
            }
            Err(e) => {
                eprintln!("{}:{} {:?}", file!(), line!(), e);
            }
        }
        ret_val
    }

}

// These methods are run in the context of the client connection, on the server.
impl Oliana for OlianaServer {
    async fn generate_text_begin(mut self, _: context::Context, system_prompt: String, user_prompt: String) -> String {

        if let Ok(ref mut token_generation_complete_wg) = self.token_generation_complete.write() {
            **token_generation_complete_wg = false;
        }

        if let Ok(ref mut generate_text_next_token_i_wg) = self.generate_text_next_token_i.write() {
            **generate_text_next_token_i_wg = 0;
        }

        if let Err(e) = self.increment_to_next_free_text_input_nonce().await {
            eprintln!("[ increment_to_next_free_text_input_nonce ] {:?}", e);
            return format!("[ increment_to_next_free_text_input_nonce ] {:?}", e);
        }

        let input_data = serde_json::json!({
            "system_prompt": system_prompt,
            "user_prompt": user_prompt
        });
        let input_data_s = input_data.to_string();

        let current_text_input_json = self.get_current_text_input_json_path();

        let response_txt_file = self.get_current_text_output_txt_path();
        if response_txt_file.exists() {
            if let Err(e) = tokio::fs::remove_file(response_txt_file).await {
                eprintln!("[ tokio::fs::remove_file ] {:?}", e);
                return format!("[ tokio::fs::remove_file ] {:?}", e);
            }
        }

        if let Err(e) = tokio::fs::write(current_text_input_json, input_data_s.as_bytes()).await {
            eprintln!("[ tokio::fs::write ] {:?}", e);
            return format!("[ tokio::fs::write ] {:?}", e);
        }

        String::new()
    }

    async fn generate_text_next_token(mut self, _: context::Context) -> Option<String> {
        // Right now we just wait for get_current_text_output_txt_path() to be created + return one giant chunk, but eventually Oliana-Text should iteratively update the file
        // so we can poll & return a streamed response.
        let response_txt_file = self.get_current_text_output_txt_path();
        while ! response_txt_file.exists() {
            tokio::time::sleep( tokio::time::Duration::from_millis(200) ).await;
        }

        eprintln!("oliana_server is Reading from response_txt_file = {:?}; self.generate_text_next_token_i = {}", response_txt_file.to_string_lossy(), self.read_generate_text_next_token_i() );

        if self.read_generate_text_next_token_i() == 0 {
            if let Ok(ref mut generate_text_next_token_i_wg) = self.generate_text_next_token_i.write() {
                **generate_text_next_token_i_wg = 1; // mark done, so we return None on next call. Janky asf, pls remove soon!
            }

            if let Ok(file_bytes) = tokio::fs::read(response_txt_file).await {
                if let Ok(the_string) = std::str::from_utf8(&file_bytes) {
                    return Some(the_string.to_string());
                }
            }

            None
        }
        else {
            None
        }

        /*
        // We poll until either self.token_generation_complete or the vec has enough tokens to return the next one
        while !self.token_generation_complete && self.generate_text_next_token_i < self.generated_text_tokens.len()-1 {
            tokio::time::sleep( tokio::time::Duration::from_millis(200) ).await;
        }

        let token = self.generated_text_tokens.get(self.generate_text_next_token_i);
        if token.is_some() {
            self.generate_text_next_token_i += 1;
        }
        return token.cloned();
        */
    }

}




