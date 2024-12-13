
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
    IsqType, PagedAttentionMetaBuilder, TextMessageRole, TextMessages, TextModelBuilder,
};

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

  let model = TextModelBuilder::new("microsoft/Phi-3.5-mini-instruct".to_string())
        .with_isq(IsqType::Q8_0)
        .with_logging()
        .with_paged_attn(|| PagedAttentionMetaBuilder::default().build())?
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

                          match model.send_chat_request(messages).await.map_err(oliana_lib::eloc!()) {
                            Ok(response) => {
                                if let Some(ref reply_txt) = response.choices[0].message.content {

                                    println!("Saving {}", out_txt_file.display());

                                    tokio::fs::write(out_txt_file, reply_txt.as_bytes()).await?;

                                }
                                else {
                                    eprintln!("WARNING: response.choices[0].message.content was None!");
                                }
                            }
                            Err(e) => {
                                allowed_errors_remaining -= 1;
                                eprintln!("{:?}", e);
                            }
                          }

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
    tokio::time::sleep(std::time::Duration::from_millis(250));
  }

  Ok(())
}
