
// The ai module is responsible for presenting a high-level
// interface to the rest of the code around running LLM and Image-Generation
// models.
// For the moment we are standardizing on openvino as a runtime because it
// offers a uniform input for many types of models and allows fairly transparent runtime
// selection of GPU, CPU, and NPU compute devices

pub async fn load_ort_session(
  local_onnx_file_path: impl Into<std::path::PathBuf>,
  remote_onnx_download_url: &str
) -> Result<ort::session::Session, Box<dyn std::error::Error>> {


  let local_onnx_file_path: std::path::PathBuf = local_onnx_file_path.into();
  let local_onnx_file_path = download_file_ifne(&local_onnx_file_path, remote_onnx_download_url).await?;

  let environment = ort::environment::Environment::builder()
    .with_name("test")
    .with_log_level(ort::LoggingLevel::Verbose)
    .build().map_err(crate::utils::eloc!())?
    .into_arc();

  let mut session = ort::session::SessionBuilder::new(&environment)?
    .with_optimization_level(ort::GraphOptimizationLevel::Level1)?
    .with_intra_threads(1)?
    .with_model_from_file(local_onnx_file_path).map_err(crate::utils::eloc!())?;

  Ok(session)
}



pub async fn get_compute_device_names() -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let mut compute_device_names: Vec<String> = vec![];

  let ort_session = load_ort_session(
    crate::utils::get_cache_file("gpt2.onnx").await?,
    "https://parcel.pyke.io/v2/cdn/assetdelivery/ortrsv2/ex_models/gpt2.onnx"
  ).await?;

  let ep_cpu = ort::ExecutionProvider::CPU( ort::execution_providers::CPUExecutionProviderOptions::default() );
  if ep_cpu.is_available() {
    compute_device_names.push(format!("{}", ep_cpu.as_str() ));
  }

  let ep_cuda = ort::ExecutionProvider::CUDA( ort::execution_providers::CUDAExecutionProviderOptions::default() );
  if ep_cuda.is_available() {
    compute_device_names.push(format!("{}", ep_cuda.as_str() ));
  }

  let ep_tensor_rt = ort::ExecutionProvider::TensorRT( ort::execution_providers::TensorRTExecutionProviderOptions::default() );
  if ep_tensor_rt.is_available() {
    compute_device_names.push(format!("{}", ep_tensor_rt.as_str() ));
  }

  let ep_openvino = ort::ExecutionProvider::OpenVINO( ort::execution_providers::OpenVINOExecutionProviderOptions::default() );
  if ep_openvino.is_available() {
    compute_device_names.push(format!("{}", ep_openvino.as_str() ));
  }

  let ep_acl = ort::ExecutionProvider::ACL( ort::execution_providers::ACLExecutionProviderOptions::default() );
  if ep_acl.is_available() {
    compute_device_names.push(format!("{}", ep_acl.as_str() ));
  }

  let ep_onednn = ort::ExecutionProvider::OneDNN( ort::execution_providers::OneDNNExecutionProviderOptions::default() );
  if ep_onednn.is_available() {
    compute_device_names.push(format!("{}", ep_onednn.as_str() ));
  }

  let ep_coreml = ort::ExecutionProvider::CoreML( ort::execution_providers::CoreMLExecutionProviderOptions::default() );
  if ep_coreml.is_available() {
    compute_device_names.push(format!("{}", ep_coreml.as_str() ));
  }

  let ep_directml = ort::ExecutionProvider::DirectML( ort::execution_providers::DirectMLExecutionProviderOptions::default() );
  if ep_directml.is_available() {
    compute_device_names.push(format!("{}", ep_directml.as_str() ));
  }

  let ep_nnapi = ort::ExecutionProvider::NNAPI( ort::execution_providers::NNAPIExecutionProviderOptions::default() );
  if ep_nnapi.is_available() {
    compute_device_names.push(format!("{}", ep_nnapi.as_str() ));
  }

  let ep_rocm = ort::ExecutionProvider::ROCm( ort::execution_providers::ROCmExecutionProviderOptions::default() );
  if ep_rocm.is_available() {
    compute_device_names.push(format!("{}", ep_rocm.as_str() ));
  }

  // TODO paste the others in here?

  Ok(compute_device_names)
}

pub async fn run_oneshot_llm_prompt(prompt_txt: &str) -> Result<String, Box<dyn std::error::Error>> {
  use rand::prelude::*;

  let mut reply = String::new();

  let ort_session = load_ort_session(
    crate::utils::get_cache_file("gpt2.onnx").await?,
    "https://parcel.pyke.io/v2/cdn/assetdelivery/ortrsv2/ex_models/gpt2.onnx"
  ).await?;

  let tokenizer_json_f = download_file_ifne(
    crate::utils::get_cache_file("gpt2-tokenizer.json").await?,
    "https://huggingface.co/openai-community/gpt2/raw/main/tokenizer.json"
  ).await?;

  let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_json_f).map_err(crate::utils::eloc_str!())?;
  let tokens = tokenizer.encode(prompt_txt, false).unwrap();
  let mut tokens = std::sync::Arc::new(tokens.get_ids().iter().map(|i| *i as i64).collect::<Vec<_>>().into_boxed_slice());

  /// Max tokens to generate
  const GEN_TOKENS: i32 = 90;

  /// Top_K -> Sample from the k most likely next tokens at each step. Lower k focuses on higher probability tokens.
  const TOP_K: usize = 5;

  let mut rng = rand::thread_rng();

  for _ in 0..GEN_TOKENS {
    // Raw tensor construction takes a tuple of (dimensions, data).
    // The model expects our input to have shape [B, _, S]
    let input = (vec![1, 1, tokens.len() as i64], std::sync::Arc::clone(&tokens));
    let outputs = ort_session.run(ort::inputs![input]?)?;
    let (dim, mut probabilities) = outputs["output1"].try_extract_raw_tensor()?;

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

  Ok(reply)
}




pub async fn run_oneshot_ai_img_prompt(prompt_txt: &str, out_file_path: &str) -> Result<String, Box<dyn std::error::Error>> {



  Ok(out_file_path.to_string())
}



pub async fn download_file_ifne(
  local_file_path: impl Into<std::path::PathBuf>,
  remote_download_url: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {

  let local_file_path = local_file_path.into();

  if !tokio::fs::try_exists(&local_file_path).await? {
    eprintln!("Downloading {} to {}", remote_download_url, &local_file_path.to_string_lossy() );
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
    eprintln!("Found already-downloaded file {}", &local_file_path.to_string_lossy() );
  }

  Ok(local_file_path)
}
