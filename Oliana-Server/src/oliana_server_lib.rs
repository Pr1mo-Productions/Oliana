#![allow(unused_imports, unused_variables, unused_mut)]

use tokio::io::AsyncReadExt;
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

    /// Runs an AI model and returns immediately; callers should wait on generate_image_get_result() to read a .png vector of bytes back
    async fn generate_image_begin(prompt: String, negative_prompt: String, guidance_scale: f32, num_inference_steps: u32) -> String;
    /// Waits until image has completed and returns result.
    async fn generate_image_get_result() -> Vec<u8>;

   /// Reads PCI data from the host the server is running on & returns a list of hardware attached
    async fn fetch_pci_hw_device_names() -> Vec<String>;

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
    pub generate_text_next_byte_i: std::sync::Arc<std::sync::RwLock<usize>>, // Keeps track of how far into the output .txt file we have read for streaming purposes

    pub image_input_nonce: std::sync::Arc<std::sync::RwLock<usize>>,
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
            generate_text_next_byte_i: std::sync::Arc::new(std::sync::RwLock::new( 0 )),

            image_input_nonce: std::sync::Arc::new(std::sync::RwLock::new( 0 )),

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
    pub fn get_current_text_output_done_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.ai_workdir_text).join(format!("{}.done", self.read_text_input_nonce()))
    }

    pub fn read_generate_text_next_byte_i(&self) -> usize {
        let mut ret_val: usize = 0;
        match self.generate_text_next_byte_i.read() {
            Ok(generate_text_next_byte_i_rg) => {
                ret_val = *generate_text_next_byte_i_rg;
            }
            Err(e) => {
                eprintln!("{}:{} {:?}", file!(), line!(), e);
            }
        }
        ret_val
    }


    pub fn read_image_input_nonce(&self) -> usize {
        let mut ret_val: usize = 0;
        match self.image_input_nonce.read() {
            Ok(image_input_nonce_rg) => {
                ret_val = *image_input_nonce_rg;
            }
            Err(e) => {
                eprintln!("{}:{} {:?}", file!(), line!(), e);
            }
        }
        ret_val
    }

    pub async fn increment_to_next_free_image_input_nonce(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        while tokio::fs::try_exists( self.get_current_image_input_json_path() ).await? {
            if let Ok(ref mut image_input_nonce_wg) = self.image_input_nonce.write() {
                **image_input_nonce_wg += 1;
            }
        }
        Ok(self.read_image_input_nonce())
    }
    pub fn get_current_image_input_json_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.ai_workdir_images).join(format!("{}.json", self.read_image_input_nonce()))
    }
    pub fn get_current_image_output_png_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.ai_workdir_images).join(format!("{}.png", self.read_image_input_nonce()))
    }
    pub fn get_current_image_output_txt_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.ai_workdir_images).join(format!("{}.txt", self.read_image_input_nonce()))
    }


}

// These methods are run in the context of the client connection, on the server.
impl Oliana for OlianaServer {
    async fn generate_text_begin(mut self, _: context::Context, system_prompt: String, user_prompt: String) -> String {

        if let Ok(ref mut generate_text_next_byte_i_wg) = self.generate_text_next_byte_i.write() {
            **generate_text_next_byte_i_wg = 0;
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

        let mut remaining_polls_before_give_up: usize = 3 * 10; // 3 seconds worth at 10 polls/sec
        while !response_txt_file.exists() && remaining_polls_before_give_up > 0 {
            tokio::time::sleep( tokio::time::Duration::from_millis(100) ).await;
            remaining_polls_before_give_up -= 1;
        }
        if !response_txt_file.exists() {
            return None;
        }

        let response_done_file = self.get_current_text_output_done_path();

        // Wait until the file's size is > self.read_generate_text_next_byte_i()
        let mut remaining_polls_before_give_up: usize = 3 * 10; // 3 seconds worth at 10 polls/sec
        loop {
            let next_byte_i = self.read_generate_text_next_byte_i();
            if let Ok(file_bytes) = tokio::fs::read(&response_txt_file).await {
                if file_bytes.len() < next_byte_i {
                    return None; // Somehow the file was truncated! .len() should always grow; it is allowed to be == next_byte_i.
                }
                if let Ok(the_string) = std::str::from_utf8(&file_bytes[next_byte_i..]) {

                    // Update the index we know we have read to to file_bytes.len()
                    match self.generate_text_next_byte_i.write() {
                        Ok(mut generate_text_next_byte_i_wg) => {
                            *generate_text_next_byte_i_wg = file_bytes.len();
                        }
                        Err(e) => {
                            eprintln!("{}:{} {:?}", file!(), line!(), e);
                        }
                    }

                    // It's possible to read 0 new bytes, in which case we do NOT want to return empty string; instead we fall down to the `response_done_file.exists() || remaining_polls_before_give_up < 1` check below.
                    if the_string.len() > 0 {
                        return Some(the_string.to_string());
                    }
                }
            }
            if response_done_file.exists() || remaining_polls_before_give_up < 1 { // What we just read must be the remaining bytes, because .done is created AFTER a write to .txt
                break;
            }
            tokio::time::sleep( tokio::time::Duration::from_millis(100) ).await;
            remaining_polls_before_give_up -= 1;
        }
        return None;
    }

    async fn generate_image_begin(mut self, _: tarpc::context::Context, prompt: String, negative_prompt: String, guidance_scale: f32, num_inference_steps: u32) -> std::string::String {
        if let Err(e) = self.increment_to_next_free_image_input_nonce().await {
            eprintln!("[ increment_to_next_free_image_input_nonce ] {:?}", e);
            return format!("[ increment_to_next_free_image_input_nonce ] {:?}", e);
        }

        let input_data = serde_json::json!({
            "prompt": prompt,
            "negative_prompt": negative_prompt,
            "guidance_scale": guidance_scale,
            "num_inference_steps": num_inference_steps,
        });
        let input_data_s = input_data.to_string();

        let current_text_input_json = self.get_current_image_input_json_path();

        let response_txt_file = self.get_current_image_output_txt_path();
        if response_txt_file.exists() {
            if let Err(e) = tokio::fs::remove_file(response_txt_file).await {
                eprintln!("[ tokio::fs::remove_file ] {:?}", e);
                return format!("[ tokio::fs::remove_file ] {:?}", e);
            }
        }

        let response_png_file = self.get_current_image_output_png_path();
        if response_png_file.exists() {
            if let Err(e) = tokio::fs::remove_file(response_png_file).await {
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

    async fn generate_image_get_result(self, _: tarpc::context::Context) -> Vec<u8> {
        let mut result_bytes: Vec<u8> = Vec::with_capacity(1024 * 1024);

        let response_txt_file = self.get_current_image_output_txt_path();
        let response_png_file = self.get_current_image_output_png_path();

        let mut remaining_polls_before_give_up: usize = 24 * 10; // 24 seconds worth at 10 polls/sec
        while !response_txt_file.exists() && !response_png_file.exists() && remaining_polls_before_give_up > 1 {
            tokio::time::sleep( tokio::time::Duration::from_millis(100) ).await;
            remaining_polls_before_give_up -= 1;
        }

        if response_png_file.exists() {
            // Just because it _exists_ doesn't mean we're done writing to it. Give the OS a tick to flush writes and continue when 100ms elapses w/ identical length values for the file
            remaining_polls_before_give_up = 4 * 10; // 4 seconds at 10 polls/sec
            let mut last_file_len: u64 = 0;
            while remaining_polls_before_give_up > 1 {
                tokio::time::sleep( tokio::time::Duration::from_millis(100) ).await;
                let mut this_file_len: u64 = 1;
                if let Ok(mut metadata) = tokio::fs::metadata(&response_png_file).await {
                    this_file_len = metadata.len();
                }
                if this_file_len == last_file_len {
                    break; // Success!
                }
                last_file_len = this_file_len;
                remaining_polls_before_give_up -= 1;
            }
            tokio::time::sleep( tokio::time::Duration::from_millis(100) ).await;

            if let Ok(mut fd) = tokio::fs::File::open(&response_png_file).await {
                if let Err(e) = fd.read_to_end(&mut result_bytes).await {
                    eprintln!("{}:{} {:?}", file!(), line!(), e);
                }
            }
        }

        if response_txt_file.exists() {
            let response_err_msg = std::fs::read_to_string(&response_txt_file).unwrap_or_else(|_| String::new());
            eprintln!("Got error from Oliana-Images: {:?}", response_err_msg);
        }

        return result_bytes;
    }

    async fn fetch_pci_hw_device_names(self, _: tarpc::context::Context) -> Vec<String> {
        let mut result = vec![];
        match pci_info::PciInfo::enumerate_pci() {
            Ok(pcie_devices) => {
                let pcie_database: Option<pciid_parser::Database> = if let Ok(db) = pciid_parser::Database::read() { Some(db) } else { None };
                for device in pcie_devices {
                    match device {
                        Ok(device) => {
                            match device.device_iface() {
                                Ok(iface) => {
                                    if iface == pci_info::pci_enums::PciDeviceInterfaceFunc::DisplayController_VgaCompatible_Vga { // It's a GPU!
                                        if let Some(ref db) = pcie_database {
                                            let vendor_id = format!("{:x}", device.vendor_id());
                                            let device_id = format!("{:x}", device.device_id());
                                            let info = db.get_device_info(
                                                vendor_id.as_str(), device_id.as_str(), "", ""
                                            );
                                            result.push(format!("{} {}",
                                                simplify_pci_dev_name(info.vendor_name.unwrap_or_else(|| "UNK".into())),
                                                simplify_pci_dev_name(info.device_name.unwrap_or_else(|| "UNK".into()))
                                            ));
                                        }
                                        else {
                                            result.push(format!("[ NO PCI DATABASE ] {:?}", device));
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{}:{} {:?}", file!(), line!(), e);
                                    result.push(format!("{:?}", e));
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("{}:{} {:?}", file!(), line!(), e);
                            result.push(format!("{:?}", e));
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}:{} {:?}", file!(), line!(), e);
                result.push(format!("{:?}", e));
            }
        }
        return result;
    }
}

fn simplify_pci_dev_name(name: &str) -> String {
    let name = name.replace("Corporation ", "");
    let name = name.replace(" Corporation", "");
    let name = name.replace("Corporation", "");
    let name = name.replace(" Graphics", "");
    let name = name.replace("Advanced Micro Devices ", "AMD ");
    let name = name.replace("Advanced Micro Devices,", "AMD,");
    let name = name.replace("  ", " ");
    return name;
}





