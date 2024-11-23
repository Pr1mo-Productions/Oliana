
// The ai module is responsible for presenting a high-level
// interface to the rest of the code around running LLM and Image-Generation
// models.
// For the moment we are standardizing on openvino as a runtime because it
// offers a uniform input for many types of models and allows fairly transparent runtime
// selection of GPU, CPU, and NPU compute devices

fn load_ort_session() -> Result<ort::session::Session, Box<dyn std::error::Error>> {

  let environment = ort::environment::Environment::builder()
    .with_name("test")
    .with_log_level(ort::LoggingLevel::Verbose)
    .build().map_err(crate::utils::eloc!())?
    .into_arc();

  let mut session = ort::session::SessionBuilder::new(&environment)?
    .with_optimization_level(ort::GraphOptimizationLevel::Level1)?
    .with_intra_threads(1)?
    .with_model_from_file("squeezenet.onnx").map_err(crate::utils::eloc!())?;

  Ok(session)
}



pub fn get_compute_device_names() -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let mut compute_device_names: Vec<String> = vec![];

  let ort_session = load_ort_session()?;


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

pub fn run_oneshot_llm_prompt(prompt_txt: &str) -> Result<String, Box<dyn std::error::Error>> {
  let mut reply = String::new();



  Ok(reply)
}




pub fn run_oneshot_ai_img_prompt(prompt_txt: &str, out_file_path: &str) -> Result<String, Box<dyn std::error::Error>> {



  Ok(out_file_path.to_string())
}



