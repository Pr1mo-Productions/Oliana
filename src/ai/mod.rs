
// The ai module is responsible for presenting a high-level
// interface to the rest of the code around running LLM and Image-Generation
// models.
// For the moment we are standardizing on openvino as a runtime because it
// offers a uniform input for many types of models and allows fairly transparent runtime
// selection of GPU, CPU, and NPU compute devices

pub async fn get_compute_device_names(cli_args: &crate::cli::Args) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  use ort::ExecutionProvider;


  let mut compute_device_names: Vec<String> = vec![];

  let ort_session = load_ort_session(
    cli_args,
    crate::utils::get_cache_file("gpt2.onnx").await?,
    "https://parcel.pyke.io/v2/cdn/assetdelivery/ortrsv2/ex_models/gpt2.onnx"
  ).await?;

  let ep_cpu = ort::CPUExecutionProvider::default();
  if ep_cpu.is_available()? {
    compute_device_names.push(format!("{}", ep_cpu.as_str() ));
  }

  let ep_cuda = ort::CUDAExecutionProvider::default();
  if ep_cuda.is_available()? {
    compute_device_names.push(format!("{}", ep_cuda.as_str() ));
  }

  let ep_tensor_rt = ort::TensorRTExecutionProvider::default();
  if ep_tensor_rt.is_available()? {
    compute_device_names.push(format!("{}", ep_tensor_rt.as_str() ));
  }

  let ep_openvino = ort::OpenVINOExecutionProvider::default();
  if ep_openvino.is_available()? {
    compute_device_names.push(format!("{}", ep_openvino.as_str() ));
  }

  let ep_acl = ort::ACLExecutionProvider::default();
  if ep_acl.is_available()? {
    compute_device_names.push(format!("{}", ep_acl.as_str() ));
  }

  let ep_onednn = ort::OneDNNExecutionProvider::default();
  if ep_onednn.is_available()? {
    compute_device_names.push(format!("{}", ep_onednn.as_str() ));
  }

  let ep_coreml = ort::CoreMLExecutionProvider::default();
  if ep_coreml.is_available()? {
    compute_device_names.push(format!("{}", ep_coreml.as_str() ));
  }

  let ep_directml = ort::DirectMLExecutionProvider::default();
  if ep_directml.is_available()? {
    compute_device_names.push(format!("{}", ep_directml.as_str() ));
  }

  let ep_nnapi = ort::NNAPIExecutionProvider::default();
  if ep_nnapi.is_available()? {
    compute_device_names.push(format!("{}", ep_nnapi.as_str() ));
  }

  let ep_rocm = ort::ROCmExecutionProvider::default();
  if ep_rocm.is_available()? {
    compute_device_names.push(format!("{}", ep_rocm.as_str() ));
  }

  // TODO paste the others in here?

  Ok(compute_device_names)
}

pub async fn run_oneshot_llm_prompt(cli_args: &crate::cli::Args, prompt_txt: &str) -> Result<String, Box<dyn std::error::Error>> {
  use rand::prelude::*;
  use rand::SeedableRng;

  let mut reply = String::new();

  #[cfg(all(feature = "llm_ollama", feature = "llm_ort"))]
  compile_error!("Do NOT specify both feature llm_ollama and llm_ort at the same time. They are mutually exclusive and only one should be specified!");

  // Either the feature llm_ort was specified, OR neither llm_ort or llm_ollama was specified. (AKA this is the default impl)
  #[cfg(any(feature = "llm_ort", all(not(feature = "llm_ort"), not(feature = "llm_ollama"))))]
  {
    if cli_args.verbose > 0 {
      eprintln!("[ Info ] Using LLM runtime ORT (Rust ONNX bindings)");
    }

    /*let ort_session = if let Some(user_specified_onnx_file) = &cli_args.llm_onnx_file {
      load_ort_session(
        cli_args,
        user_specified_onnx_file,
        ""
      ).await?
    } else {
      load_ort_session(
        cli_args,
        crate::utils::get_cache_file("gpt2.onnx").await.map_err(crate::utils::eloc!())?,
        "https://parcel.pyke.io/v2/cdn/assetdelivery/ortrsv2/ex_models/gpt2.onnx"
      ).await.map_err(crate::utils::eloc!())?
    };*/

    let ort_inferencer = if let Some(user_specified_onnx_file_or_dir) = &cli_args.llm_onnx_file {
      ORTInferencer::init_from_local_file_or_folder(cli_args, user_specified_onnx_file_or_dir).await?
    }
    else {
      let local_onnx_file_path = crate::utils::get_cache_file("gpt2.onnx").await.map_err(crate::utils::eloc!())?;
      let local_onnx_file_path = download_file_ifne(
        cli_args,
        &local_onnx_file_path,
        "https://parcel.pyke.io/v2/cdn/assetdelivery/ortrsv2/ex_models/gpt2.onnx" // TODO maybe read from env & dynamically adjust file path upstairs? idk, GPT2 kinda sux.
      ).await?;
      ORTInferencer::init_from_local_file_or_folder(cli_args, local_onnx_file_path).await?
    };

    let tokenizer_json_f = if let Some(user_specified_tokenizer_json_file) = &cli_args.llm_tokenizer_json_file {
      user_specified_tokenizer_json_file.into()
    }
    else {
      download_file_ifne(
        cli_args,
        crate::utils::get_cache_file("gpt2-tokenizer.json").await.map_err(crate::utils::eloc!())?,
        "https://huggingface.co/openai-community/gpt2/raw/main/tokenizer.json"
      ).await.map_err(crate::utils::eloc!())?
    };

    let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_json_f).map_err(crate::utils::eloc_str!())?;
    let tokens = tokenizer.encode(prompt_txt, false).unwrap();
    let mut tokens = std::sync::Arc::new(tokens.get_ids().iter().map(|i| *i as i64).collect::<Vec<_>>().into_boxed_slice());

    /// Max tokens to generate
    const GEN_TOKENS: i32 = 90;

    /// Top_K -> Sample from the k most likely next tokens at each step. Lower k focuses on higher probability tokens.
    const TOP_K: usize = 5;

    let mut rng: Box<dyn rand::RngCore> = if let Some(random_seed) = cli_args.random_seed {
      Box::new(rand::rngs::StdRng::seed_from_u64(random_seed as u64))
    } else {
      Box::new(rand::thread_rng())
    };

    for _ in 0..GEN_TOKENS {
      // Raw tensor construction takes a tuple of (dimensions, data).
      // !!! only true for gpt2.onnx >> !!! The model expects our input to have shape [B, _, S]

      let outputs = ort_inferencer.run_inference_step(std::sync::Arc::clone(&tokens)).await.map_err(crate::utils::eloc!())?;
      let (dim, mut probabilities) = outputs["output1"].try_extract_raw_tensor().map_err(crate::utils::eloc!())?;

      // The output tensor will have shape [B, _, S + 1, V]
      // We want only the probabilities for the last token in this sequence, which will be the token generated by the model
      let (seq_len, vocab_size) = (dim[2] as usize, dim[3] as usize);
      probabilities = &probabilities[(seq_len - 1) * vocab_size..];

      // Sort each token by probability
      let mut probabilities: Vec<(usize, f32)> = probabilities.iter().copied().enumerate().collect();
      probabilities.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Less));

      // Sample using top-k sampling
      let token = probabilities[rng.gen_range(0..=TOP_K)].0 as i64;

      // Add our generated token to the input sequence
      let mut vec = tokens.to_vec();
      vec.push(token);
      *std::sync::Arc::make_mut(&mut tokens) = vec.into_boxed_slice();

      let token_str = tokenizer.decode(&[token as u32], true).unwrap();
      reply = format!("{}{}", reply, token_str);
      //print!("{}", token_str);
      //stdout.flush().unwrap();
    }
  }

  #[cfg(feature = "llm_ollama")]
  {
    if cli_args.verbose > 0 {
      eprintln!("[ Info ] Using LLM runtime ollama (requires ollama.exe installed)");
    }
    // Try to connect to default, if cannot spawn "ollama serve"
    let ollama = ollama_rs::Ollama::default();

    match ollama.list_local_models().await {
      Ok(local_models) => {
        if cli_args.verbose > 1 {
          eprintln!("Ollama already running, models = {:#?}", local_models);
        }
      }
      Err(e) => {
        if cli_args.verbose > 1 {
          eprintln!("{:#?}", crate::utils::LocatedError { inner: Box::new(e), file: file!(), line: line!(), column: column!(), addtl_msg: String::new() });
        }

        eprintln!("Executing 'ollama serve' as a background process...");

        tokio::process::Command::new("ollama")
          .args(&["serve"])
          .kill_on_drop(false) // Prevents tokio from reaping process on Drop
          .spawn().map_err(crate::utils::eloc!())?;

        // Delay for 750ms or so
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
      }
    }

    let local_models = ollama.list_local_models().await.map_err(crate::utils::eloc!())?;
    // eprintln!("Ollama models = {:#?}", local_models);

    /*let qwen2_5_7b_model_file = download_file_ifne(
      cli_args,
      crate::utils::get_cache_file("qwen2_5_7b.Modelfile").await?,
      "https://huggingface.co/openai-community/gpt2/raw/main/tokenizer.json"
    ).await?;*/
    // ^^ todo research so we can control our own downloads

    const OLLAMA_MODEL_NAME: &'static str = "qwen2.5:7b";

    match ollama.show_model_info(OLLAMA_MODEL_NAME.to_string()).await {
      Ok(model_info) => { /* unused */ },
      Err(e) => {
        if cli_args.verbose > 1 {
          eprintln!("{:#?}", crate::utils::LocatedError { inner: Box::new(e), file: file!(), line: line!(), column: column!(), addtl_msg: String::new() });
        }
        // Spawn off a download
        eprintln!("Telling ollama to pull the model {}...", OLLAMA_MODEL_NAME);
        ollama.pull_model(OLLAMA_MODEL_NAME.to_string(), true).await?;
        eprintln!("Done pulling {}!", OLLAMA_MODEL_NAME);
      }
    }

    let res = ollama.generate(ollama_rs::generation::completion::request::GenerationRequest::new(OLLAMA_MODEL_NAME.to_string(), prompt_txt.to_string())).await;

    match res {
      Ok(res) => {
        reply = res.response;
      }
      Err(e) => {
        reply = format!("{:#?}", crate::utils::LocatedError { inner: Box::new(e), file: file!(), line: line!(), column: column!(), addtl_msg: String::new() });
      }
    }

  }

  Ok(reply)
}




pub async fn run_oneshot_ai_img_prompt(cli_args: &crate::cli::Args, prompt_txt: &str, out_file_path: &str) -> Result<String, Box<dyn std::error::Error>> {



  Ok(out_file_path.to_string())
}



pub async fn download_file_ifne(
  cli_args: &crate::cli::Args,
  local_file_path: impl Into<std::path::PathBuf>,
  remote_download_url: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {

  let local_file_path = local_file_path.into();

  if !tokio::fs::try_exists(&local_file_path).await? {
    if cli_args.verbose > 0 {
      eprintln!("Downloading {} to {}", remote_download_url, &local_file_path.to_string_lossy() );
    }
    if remote_download_url.len() < 1 {
      return Err(format!("The file {:?} does not exist and no URL was passed to download it!", &local_file_path).into());
    }

    let mut downloader = downloader::Downloader::builder()
          .download_folder( local_file_path.parent().ok_or_else(|| return "No Parent Directory for passed file to be downloaded!" ).map_err(crate::utils::eloc!())? )
          .parallel_requests(2)
          .build()?;
    let dl_file_name_osstr = local_file_path.file_name().ok_or_else(|| return "No File Name for passed file to be downloaded!" ).map_err(crate::utils::eloc!())?;
    let dl_file_name_string = dl_file_name_osstr.to_string_lossy().into_owned();

    let dl = downloader::Download::new(remote_download_url)
                .file_name( &std::path::Path::new( &dl_file_name_string ) )
                .progress(std::sync::Arc::new(
                  crate::utils::DownloadProgressReporter::new()
                ));

    let result = downloader.async_download(&[dl]).await?;

  }
  else {
    if cli_args.verbose > 0 {
      eprintln!("Found already-downloaded file {}", &local_file_path.to_string_lossy() );
    }
  }

  Ok(local_file_path)
}


pub async fn load_ort_session(
  cli_args: &crate::cli::Args,
  local_onnx_file_path: impl Into<std::path::PathBuf>,
  remote_onnx_download_url: &str
) -> Result<ort::Session, Box<dyn std::error::Error>> {


  let local_onnx_file_path: std::path::PathBuf = local_onnx_file_path.into();
  let local_onnx_file_path = download_file_ifne(cli_args, &local_onnx_file_path, remote_onnx_download_url).await?;

  let mut session = ort::Session::builder()?
    .with_optimization_level(ort::GraphOptimizationLevel::Level1)?
    .with_intra_threads(1)?
    .commit_from_file(local_onnx_file_path).map_err(crate::utils::eloc!())?;

  Ok(session)
}


// Used to keep track of custom logic necessary for various types of ONNX model layouts
pub enum ORTInferencer {
  DirectModel      { session: ort::Session },
  LanguageToLogits { language_session: ort::Session, logits_session: ort::Session },
}

impl ORTInferencer {
  pub async fn init_from_local_file_or_folder(cli_args: &crate::cli::Args, pb:  impl Into<std::path::PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
    let pb = pb.into();
    let pb_as_a_string: String = pb.clone().into_os_string().into_string().map_err(crate::utils::eloc_str!())?; // Yes this is a type-management sin, no I don't care we can keep 2x paths and an error-to-string around.

    if pb.is_file() && pb_as_a_string.to_lowercase().ends_with(".onnx") {
      let mut session = ort::Session::builder()?
        .with_optimization_level(ort::GraphOptimizationLevel::Level1)?
        .with_intra_threads(1)?
        .commit_from_file(pb_as_a_string).map_err(crate::utils::eloc!())?;
      Ok(
        ORTInferencer::DirectModel {
          session: session
        }
      )
    }
    else if pb.is_dir() {
      // Scan for the first ".onnx" file w/ "language" in name
      let mut language_onnx_path: String = String::new();
      let mut logits_onnx_path: String = String::new();

      let mut dir_reader = tokio::fs::read_dir(&pb).await?;
      loop {
        if let Some(dir_f) = dir_reader.next_entry().await? {
          let dir_f_str = dir_f.file_name().into_string().map_err(crate::utils::eloc_str!())?;
          let lower_dir_f_str = dir_f_str.to_lowercase();
          if lower_dir_f_str.ends_with(".onnx") && lower_dir_f_str.contains("language") {
            language_onnx_path = dir_f.path().into_os_string().into_string().map_err(crate::utils::eloc_str!())?;
            break;
          }
        }
        else {
          break;
        }
      }
      let mut dir_reader = tokio::fs::read_dir(&pb).await?;
      loop {
        if let Some(dir_f) = dir_reader.next_entry().await? {
          let dir_f_str = dir_f.file_name().into_string().map_err(crate::utils::eloc_str!())?;
          let lower_dir_f_str = dir_f_str.to_lowercase();
          if lower_dir_f_str.ends_with(".onnx") && lower_dir_f_str.contains("logit") {
            logits_onnx_path = dir_f.path().into_os_string().into_string().map_err(crate::utils::eloc_str!())?;
            break;
          }
        }
        else {
          break;
        }
      }

      if language_onnx_path.len() < 1 {
        return Err(format!("The directory {:?} is misisng a language-model.onnx file, cannot construct an ORT session from it!", &pb_as_a_string).into());
      }
      if logits_onnx_path.len() < 1 {
        return Err(format!("The directory {:?} is misisng a logits-model.onnx file, cannot construct an ORT session from it!", &pb_as_a_string).into());
      }

      if cli_args.verbose > 0 {
        eprintln!("Selected language_onnx_path = {:?}", language_onnx_path);
        eprintln!("Selected logits_onnx_path = {:?}", logits_onnx_path);
      }

      let mut language_session = ort::Session::builder()?
        .with_optimization_level(ort::GraphOptimizationLevel::Level1)?
        .with_intra_threads(1)?
        .commit_from_file(language_onnx_path).map_err(crate::utils::eloc!())?;

      let mut logits_session = ort::Session::builder()?
        .with_optimization_level(ort::GraphOptimizationLevel::Level1)?
        .with_intra_threads(1)?
        .commit_from_file(logits_onnx_path).map_err(crate::utils::eloc!())?;

      Ok(
        ORTInferencer::LanguageToLogits {
          language_session: language_session,
          logits_session: logits_session
        }
      )
    }
    else {
      Err(format!("The file {:?} is neither a file or a directory, cannot construct an ORT session from it!", &pb_as_a_string).into())
    }
  }

  pub async fn run_inference_step(&self, tokens: std::sync::Arc<Box<[i64]>>)  -> Result<ort::SessionOutputs, Box<dyn std::error::Error>>  {
    match self {
      ORTInferencer::DirectModel { session } => {
        let input = (vec![1, 1, tokens.len() as i64], std::sync::Arc::clone(&tokens));
        let outputs = session.run(ort::inputs![input].map_err(crate::utils::eloc!())?)?;
        Ok(outputs)
      }
      ORTInferencer::LanguageToLogits { language_session, logits_session  } => {

        // Lotsa TODOs here
        eprintln!("tokens.len() = {}", tokens.len());

        let input = (vec![tokens.len() as i64, 1], std::sync::Arc::clone(&tokens));
        let outputs = language_session.run(ort::inputs![input].map_err(crate::utils::eloc!())?).map_err(crate::utils::eloc!())?;

        let input = (vec![tokens.len() as i64, tokens.len() as i64 * tokens.len() as i64], std::sync::Arc::clone(&tokens));
        let outputs = logits_session.run(ort::inputs![input].map_err(crate::utils::eloc!())?).map_err(crate::utils::eloc!())?;

        Ok(outputs)
      }
    }
  }

}

