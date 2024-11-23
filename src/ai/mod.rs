
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

  if !tokio::fs::try_exists(&local_onnx_file_path).await? {
    eprintln!("Downloading {} to {}", remote_onnx_download_url, &local_onnx_file_path.to_string_lossy() );
    let mut downloader = downloader::Downloader::builder()
          .download_folder( local_onnx_file_path.parent().ok_or_else(|| return "No Parent Directory for passed file to be downloaded!" ).map_err(crate::utils::eloc!())? )
          .parallel_requests(2)
          .build()?;
    let dl_file_name_osstr = local_onnx_file_path.file_name().ok_or_else(|| return "No File Name for passed file to be downloaded!" ).map_err(crate::utils::eloc!())?;
    let dl_file_name_string = dl_file_name_osstr.to_string_lossy().into_owned();

    let dl = downloader::Download::new(remote_onnx_download_url)
                .file_name( &std::path::Path::new( &dl_file_name_string ) )
                .progress(std::sync::Arc::new(
                  crate::utils::DownloadProgressReporter::new()
                ));

    let result = downloader.async_download(&[dl]).await?;

  }
  else {
    eprintln!("Found already-downloaded file {}", &local_onnx_file_path.to_string_lossy() );
  }


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


/*
  let ov = openvino::Core::new()?;
  let mut devices = ov.available_devices()?;

  devices.sort();
  let mut explicit_device_num = std::collections::HashMap::<&openvino::DeviceType, isize>::new();

  for device in &devices {
    let dev_type_count = explicit_device_num.entry(device).or_insert(-1);
    *dev_type_count += 1;

    let mut name = format!("{:?}.{}", device, explicit_device_num[device]);
    match ov.get_property(device, &openvino::PropertyKey::DeviceFullName) {
      Ok(val) => {
        name = format!("{name} {val}");
      }
      Err(e) => {
        name = format!("{name} {e:?}");
      }
    }

    compute_device_names.push(name);
  }
*/


  Ok(compute_device_names)
}

pub async fn run_oneshot_llm_prompt(prompt_txt: &str) -> Result<String, Box<dyn std::error::Error>> {
  let mut reply = String::new();



  Ok(reply)
}




pub async fn run_oneshot_ai_img_prompt(prompt_txt: &str, out_file_path: &str) -> Result<String, Box<dyn std::error::Error>> {



  Ok(out_file_path.to_string())
}



