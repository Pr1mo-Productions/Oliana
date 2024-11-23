
// The ai module is responsible for presenting a high-level
// interface to the rest of the code around running LLM and Image-Generation
// models.
// For the moment we are standardizing on openvino as a runtime because it
// offers a uniform input for many types of models and allows fairly transparent runtime
// selection of GPU, CPU, and NPU compute devices

pub fn get_openvino_compute_device_names() -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let mut compute_device_names: Vec<String> = vec![];

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


  Ok(compute_device_names)
}



