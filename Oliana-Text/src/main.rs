
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt  = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(std::cmp::max(2, num_cpus::get_physical())) // Use all host cores, unless single-cored in which case pretend to have 2
    .thread_stack_size(8 * 1024 * 1024)
    .enable_time()
    .enable_io()
    .build()?;

  rt.block_on(async {
    if let Err(e) = main_async().await {
      eprintln!("[ main_async ] {}", e);
      std::process::exit(1);
    }
  });

  Ok(())
}

use mistralrs::{
    MemoryGpuConfig,
    IsqType, PagedAttentionMetaBuilder, TextMessageRole, TextMessages, TextModelBuilder,
};
use tokio::io::AsyncWriteExt;

async fn main_async() -> Result<(), Box<dyn std::error::Error>> {

  let args: Vec<String> = std::env::args().collect();
  let mut env_var_work_dir = std::env::var("WORK_DIR").unwrap_or("".to_string());

  if let Some(work_dir_i) = args.iter().position(|n| n == "--work-dir" || n == "--workdir") {
    if work_dir_i < args.len()-1 {
      env_var_work_dir = args[work_dir_i+1].clone();
    }
  }

  if env_var_work_dir.len() < 1 {
    eprintln!("Error, must have either WORK_DIR as an environment variable OR pass --work-dir as an argument, exiting!");
    return Ok(());
  }

  println!("");
  println!("Using {env_var_work_dir} as a work directory.");
  println!("write files named 'NAME.json' containing objects like:");
  println!(r#" {{"system_prompt": "You are an AI agent with a specialty in cooking.", "user_prompt": "Hello! How are you? I'd like to bake a pie but do not know how, please help me!", }}"#);
  println!("and wait for 'NAME.txt' to be written back from this process.");
  println!("'NAME.json' will remain post-generation, and if the file's mtime becomes newer it will be processed again with the new contents.");
  println!("If 'NAME.json' has an mtime older than this process's start time, it will not be processed.");
  println!("");

  tokio::fs::create_dir_all(&env_var_work_dir[..]).await?;

  std::env::set_var(
    "WORK_DIR", env_var_work_dir.clone()
  );

  let hf_home = oliana_lib::files::get_cache_file("Oliana-Text-hf_home").await.map_err(oliana_lib::eloc!())?;
  let hf_home = hf_home.to_string_lossy();
  tokio::fs::create_dir_all(&hf_home[..]).await?;

  eprintln!("Storing model data at {hf_home}");

  std::env::set_var(
    "HF_HOME", hf_home.to_string()
  );

  let allowed_vram_fraction: f32 = std::env::var("PER_PROC_MEM_FRACT").unwrap_or("1".to_string()).parse().unwrap_or(1.0 as f32);
  println!("PER_PROC_MEM_FRACT = {allowed_vram_fraction} (set by PER_PROC_MEM_FRACT, from 0.0 to 1.0)");

  let model = TextModelBuilder::new("microsoft/Phi-3.5-mini-instruct".to_string())
        .with_isq(IsqType::Q8_0)
        .with_logging()
        .with_paged_attn(|| PagedAttentionMetaBuilder::default()
            .with_gpu_memory(MemoryGpuConfig::Utilization(allowed_vram_fraction))
            .build()
        )?
        .build()
        .await.map_err(oliana_lib::eloc!())?;

  let our_start_time = std::time::SystemTime::now();
  let mut last_seen_mtime = std::collections::HashMap::<std::path::PathBuf, std::time::SystemTime>::new();
  let mut allowed_errors_remaining = 100;
  loop {
    let mut dir_iterator = tokio::fs::read_dir(&env_var_work_dir).await?;
    while let Some(entry) = dir_iterator.next_entry().await? {
        let entry_path = entry.path();
        if entry_path.is_file() && entry_path.extension().and_then(std::ffi::OsStr::to_str).unwrap_or("").ends_with("json") {
            // Have JSON, is it new?
            match tokio::fs::metadata(&entry_path).await {
                Ok(entry_metadata) => {
                    if let Ok(file_mtime) = entry_metadata.modified() {
                        if file_mtime > our_start_time && file_mtime > *last_seen_mtime.get(&entry_path).unwrap_or(&std::time::SystemTime::UNIX_EPOCH) {
                            // we're newer than this process's begin and we're newer than the last mtime we saw, falling back to Jan 01 1970 if never seen file before.
                            println!("Processing {}", entry_path.display());
                            last_seen_mtime.insert(entry_path.clone(), std::time::SystemTime::now());

                            let mut out_txt_file = entry_path.clone();
                            out_txt_file.set_extension("txt");
                            let out_txt_file = out_txt_file;

                            let mut out_done_file = entry_path.clone();
                            out_done_file.set_extension("done");
                            let out_done_file = out_done_file;
                            if out_done_file.exists() {
                                tokio::fs::remove_file(&out_done_file).await?;
                            }

                            // This has a Drop trait which creates the passed-in file when it is no longer in scope; combined with the error? returns below,
                            // this guarantees when the computation is done, out_done_file exists.
                            let out_done_writer = CreateFileOnDropped::new(out_done_file);

                            let input_json_text = tokio::fs::read_to_string(&entry_path).await?;
                            let input_data: serde_json::Value = serde_json::from_str(&input_json_text)?;
                            eprintln!("Read input_data = {input_json_text}");

                            let mut system_prompt = "".to_string();
                            let mut user_prompt = "".to_string();

                            if let serde_json::value::Value::Object(input_obj) = input_data {
                                if let Some(serde_json::value::Value::String(input_system_prompt)) = input_obj.get("system_prompt") {
                                    system_prompt = input_system_prompt.to_string();
                                }
                                if let Some(serde_json::value::Value::String(input_user_prompt)) = input_obj.get("user_prompt") {
                                    user_prompt = input_user_prompt.to_string();
                                }
                            }

                            let messages = TextMessages::new()
                            .add_message(
                                TextMessageRole::System,
                                &system_prompt[..],
                            )
                            .add_message(
                                TextMessageRole::User,
                                &user_prompt[..],
                            );

                            // First zero the file we write chunks to
                            tokio::fs::write(out_txt_file.as_path(), "".as_bytes()).await?;
                            // Then open in append mode
                            let mut out_txt_fd = tokio::fs::File::options()
                                                    .append(true)
                                                    .open(out_txt_file.as_path()).await?;

                            match model.stream_chat_request(messages).await.map_err(oliana_lib::eloc!()) {
                                Ok(mut response_stream) => {
                                    while let Some(ref response) = response_stream.next().await {
                                        match response {
                                            mistralrs::Response::InternalError(err) => {
                                                out_txt_fd.write_all(format!("\n{:?}\n", err).as_bytes()).await?;
                                                break;
                                            },
                                            mistralrs::Response::ValidationError(err) => {
                                                out_txt_fd.write_all(format!("\n{:#?}\n", err).as_bytes()).await?;
                                                break;
                                            },
                                            mistralrs::Response::ModelError(s, completion_response) => {
                                                out_txt_fd.write_all(format!("\n{:#?},{:#?}\n", s, completion_response).as_bytes()).await?;
                                                break;
                                            },
                                            mistralrs::Response::Done(_completion_response) => {
                                                //out_txt_fd.write_all(format!("\n{:#?}\n", completion_response).as_bytes()).await?;
                                                break;
                                            },
                                            mistralrs::Response::Chunk(chunk) => {
                                                //out_txt_fd.write_all(format!("\n{:#?}\n", chunk).as_bytes()).await?;
                                                for choice in chunk.choices.iter() {
                                                    out_txt_fd.write_all(format!("{}", choice.delta.content ).as_bytes()).await?;
                                                }
                                            },
                                            mistralrs::Response::CompletionModelError(s, completion_response) => {
                                                out_txt_fd.write_all(format!("\n{:#?},{:#?}\n", s, completion_response).as_bytes()).await?;
                                            },
                                            mistralrs::Response::CompletionDone(_completion_response) => {
                                                //out_txt_fd.write_all(format!("\n{:#?}\n", completion_response).as_bytes()).await?;
                                                break;
                                            },
                                            mistralrs::Response::CompletionChunk(chunk) => {
                                                out_txt_fd.write_all(format!("\n{:#?}\n", chunk).as_bytes()).await?;
                                            },
                                            mistralrs::Response::ImageGeneration(image_gen_response) => {
                                                out_txt_fd.write_all(format!("\n{:#?}\n", image_gen_response).as_bytes()).await?;
                                            },
                                            _unused_raw => { /* NOP */ }
                                        }
                                        out_txt_fd.flush().await?;

                                    }

                                }
                                Err(e) => {
                                    allowed_errors_remaining -= 1;
                                    eprintln!("{:?}", e);
                                    out_txt_fd.write_all(format!("\n{:#?}\n", e).as_bytes()).await?;
                                }
                            }

                            std::mem::drop(out_done_writer);

                        }

                    }
                }
                Err(e) => {
                    allowed_errors_remaining -= 1;
                    eprintln!("{:?}", e);
                }
            }
        }

    }
    if allowed_errors_remaining < 1 {
        break;
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
  }

  Ok(())
}

#[clippy::has_significant_drop]
pub struct CreateFileOnDropped {
    pub file_path: std::path::PathBuf,
}

impl CreateFileOnDropped {
    pub fn new(file_path: std::path::PathBuf) -> Self {
        Self {
            file_path: file_path
        }
    }
}

impl Drop for CreateFileOnDropped {
    fn drop(&mut self) {
        if let Err(e) = std::fs::write(self.file_path.as_path(), " ".as_bytes()) {
            eprintln!("{:?} when creating file {}", e, self.file_path.display());
        }
    }
}
