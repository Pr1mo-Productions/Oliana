
#![allow(unused_variables)]

enum InferenceType {
  CudaOnly,
  Anything
}
impl InferenceType {
  pub fn to_string(&self) -> String {
    match self {
      InferenceType::CudaOnly => "cuda-only".into(),
      InferenceType::Anything => "anything".into(),
    }
  }
}

const INFERENCE_TYPE: InferenceType = if cfg!(feature = "cuda") { InferenceType::CudaOnly } else { InferenceType::Anything };


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

use pyo3::prelude::*;
use pyo3::ffi::c_str;

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
  println!(r#" {{"prompt": "A cow jumps over the moon while fireworks explode in the air", "negative_prompt": "worst quality, low quality, ugly, duplicate, morbid, mutilated, extra fingers, mutated hands, extra limbs, cloned face, disfigured, malformed limbs, missing arms, missing legs", "guidance_scale": 3.5, "num_inference_steps": 10 }}"#);
  println!("and wait for either 'NAME.png' or 'NAME.txt' to be written back from this process.");
  println!("'NAME.json' will remain post-generation, and if the file's mtime becomes newer it will be processed again with the new contents.");
  println!("If 'NAME.json' has an mtime older than this process's start time, it will not be processed.");
  println!("");

  tokio::fs::create_dir_all(&env_var_work_dir[..]).await.map_err(oliana_lib::eloc!())?;

  std::env::set_var(
    "WORK_DIR", env_var_work_dir.clone()
  );

  let site_packages = oliana_lib::files::get_cache_file("Oliana-Images-site_packages").map_err(oliana_lib::eloc!())?;
  let site_packages = site_packages.to_string_lossy();
  tokio::fs::create_dir_all(&site_packages[..]).await.map_err(oliana_lib::eloc!())?;

  let pythonpath = std::env::join_paths(&[
    site_packages.to_string(),
    std::env::var("PYTHONPATH").unwrap_or("".to_string()),
  ]).map_err(oliana_lib::eloc!())?;

  std::env::set_var(
    "PYTHONPATH", pythonpath
  );

  // we iterate all 'lib' directories under site_packages and add them to PATH.
  let mut site_packages_lib_folders: Vec<String> = vec![];
  for entry in walkdir::WalkDir::new(&site_packages[..]).into_iter().filter_map(|e| e.ok()) {
    if let Some(file_name) = entry.path().file_name() {
      site_packages_lib_folders.push(
        entry.path().to_string_lossy().to_string()
      );
    }
  }
  // Finally add pre-existing PATH
  let pre_existing_path = std::env::var("PATH").unwrap_or("".to_string());
  let pre_existing_paths = std::env::split_paths(&pre_existing_path);
  for path in pre_existing_paths {
    site_packages_lib_folders.push(
      path.to_string_lossy().to_string()
    );
  }

  let os_path = std::env::join_paths(&site_packages_lib_folders[..]).map_err(oliana_lib::eloc!())?;

  std::env::set_var(
    "PATH", os_path
  );

  let hf_home = oliana_lib::files::get_cache_file("Oliana-Images-hf_home").map_err(oliana_lib::eloc!())?;
  let hf_home = hf_home.to_string_lossy();
  tokio::fs::create_dir_all(&hf_home[..]).await.map_err(oliana_lib::eloc!())?;

  eprintln!("Storing model data at {hf_home}");

  std::env::set_var(
    "HF_HOME", hf_home.to_string()
  );

  python_main(&site_packages, &env_var_work_dir).map_err(oliana_lib::eloc!())?;

  Ok(())
}

fn python_main(site_packages: &str, env_var_work_dir: &str) -> Result<(), Box<dyn std::error::Error>>  {
  Python::with_gil(|py| {
      let sys = py.import("sys")?;
      let version: String = sys.getattr("version").map_err(oliana_lib::eloc!())?.extract().map_err(oliana_lib::eloc!())?;

      println!("Oliana-Images is using Python {version} for processing");

      if let Err(e) = py.import("pip") {
        match py.import("ensurepip") {
          Err(e2) => {
            println!("Python likely cannot be setup; both pip and ensurepip failed to import!");
            eprintln!("{:?}", e);
            eprintln!("{:?}", e2);
            eprintln!("");
          }
          Ok(ensurepip) => {
            let ensurepip_main: Py<PyAny> = ensurepip.getattr("_main").map_err(oliana_lib::eloc!())?.into();
            if let Err(e) = ensurepip_main.call1(py, ( ) ).map_err(oliana_lib::eloc!()) {
              eprintln!("{}:{} {:?}", file!(), line!(), e);
            }
          }
        }
      }

      let pip = py.import("pip")?;
      let pip_main: Py<PyAny> = pip.getattr("main").map_err(oliana_lib::eloc!())?.into();

      if let Err(e) = py.import("torch") {
        eprintln!("{}:{} {:?}", file!(), line!(), e);
        let arg_vals = match INFERENCE_TYPE {
          InferenceType::CudaOnly =>
            vec![
              "install".to_string(), format!("--target={site_packages}"), "torch".to_string(), "torchvision".to_string(), "torchaudio".to_string(),
              "--index-url".to_string(), "https://download.pytorch.org/whl/cu124".to_string(),
            ],
          InferenceType::Anything =>
            vec![
              "install".to_string(), format!("--target={site_packages}"), "torch".to_string(), "torchvision".to_string(), "torchaudio".to_string(),
            ]
        };
        let args = (arg_vals, );
        pip_main.call1(py, args).map_err(oliana_lib::eloc!())?;
      }

      let torch = py.import("torch").map_err(oliana_lib::eloc!())?;
      eprintln!("torch = {:?}", torch);


      if let Err(e) = py.import("transformers") {
        eprintln!("{}:{} {:?}", file!(), line!(), e);
        let arg_vals = vec![
          "install".to_string(), format!("--target={site_packages}"), "transformers".to_string(),
        ];
        let args = (arg_vals, );
        pip_main.call1(py, args).map_err(oliana_lib::eloc!())?;
      }

      let transformers = py.import("transformers").map_err(oliana_lib::eloc!())?;
      eprintln!("transformers = {:?}", transformers);


      if let Err(e) = py.import("diffusers") {
        eprintln!("{}:{} {:?}", file!(), line!(), e);
        let arg_vals = vec![
          "install".to_string(), format!("--target={site_packages}"), "diffusers".to_string(),
        ];
        let args = (arg_vals, );
        pip_main.call1(py, args).map_err(oliana_lib::eloc!())?;
      }

      let diffusers = py.import("diffusers").map_err(oliana_lib::eloc!())?;
      eprintln!("diffusers = {:?}", diffusers);

      /*if let Err(e) = py.import("accelerate") {
        eprintln!("{:?}", e);
        let arg_vals = vec![
          "install".to_string(), format!("--target={site_packages}"), "accelerate".to_string(),
        ];
        let args = (arg_vals, );
        pip_main.call1(py, args)?;
      }

      let accelerate = py.import("accelerate")?;
      eprintln!("accelerate = {:?}", accelerate);*/ // ^^ accelerate is more trouble than its worth

      if let Err(e) = py.import("json5") {
        eprintln!("{:?}", e);
        let arg_vals = vec![
          "install".to_string(), format!("--target={site_packages}"), "json5".to_string(),
        ];
        let args = (arg_vals, );
        pip_main.call1(py, args).map_err(oliana_lib::eloc!())?;
      }

      let json5 = py.import("json5").map_err(oliana_lib::eloc!())?;
      eprintln!("json5 = {:?}", json5);

      let python_module = PyModule::from_code(
          py,
          c_str!(r#"
def main(env_var_work_dir, inference_type_str):
  import traceback
  import os
  import time
  import json5
  try:
    if hasattr(os, 'add_dll_directory'):
      for folder in os.environ.get('PATH', '').split(os.pathsep):
        os.add_dll_directory(folder)
  except:
    traceback.print_exc()

  import torch
  from diffusers import StableDiffusionXLPipeline, EulerDiscreteScheduler

  try:
    for i in range(torch.cuda.device_count()):
      print('We can see the CUDA device named ', torch.cuda.get_device_properties(i).name)
    if torch.cuda.device_count() < 0:
      print('NO CUDA DEVICES DETECTED!')
      raise Excepion('NO CUDA DEVICES DETECTED!')
  except:
    if 'cuda' in inference_type_str:
      raise
    # Otherwise we simply continue & rely on Torch to allocate CPU space

  try:
    fraction = float(os.environ.get('PER_PROC_MEM_FRACT', '1'))
    torch.cuda.set_per_process_memory_fraction(fraction)
    print(f'torch.cuda.set_per_process_memory_fraction({fraction}) (set by PER_PROC_MEM_FRACT, from 0.0 to 1.0)')
  except:
    traceback.print_exc()

  # You can replace the checkpoint id with several koala models as below:
  # "etri-vilab/koala-lightning-700m"

  pipe = StableDiffusionXLPipeline.from_pretrained("etri-vilab/koala-lightning-1b", torch_dtype=torch.float16)
  pipe = pipe.to("cuda")

  # Ensure sampler uses "trailing" timesteps and "sample" prediction type.
  pipe.scheduler = EulerDiscreteScheduler.from_config(
    pipe.scheduler.config, timestep_spacing="trailing"
  )

  # Now we poll env_var_work_dir forever!
  our_start_time = int(time.time())
  last_seen_mtime = dict()
  allowed_errors_remaining = 100
  while allowed_errors_remaining > 0:
    try:
      for file_name in os.listdir(env_var_work_dir):
        full_path = os.path.join(env_var_work_dir, file_name)
        if os.path.isfile(full_path):
          if full_path.casefold().endswith(".json".casefold()):
            file_mtime = os.path.getmtime(full_path)
            if file_mtime > our_start_time and last_seen_mtime.get(full_path, 0) < file_mtime:
              # We either have NOT seen this file yet or it has been updated, process it!
              file_name_no_extension, _unused_ext = os.path.splitext(file_name)
              print(f'Processing {full_path}')
              out_txt_file = os.path.join(env_var_work_dir, f'{file_name_no_extension}.txt')
              out_png_file = os.path.join(env_var_work_dir, f'{file_name_no_extension}.png')
              try:
                last_seen_mtime[full_path] = file_mtime + 1

                input_data = dict()
                with open(full_path, 'r') as fd:
                  input_data = json5.loads(fd.read())

                print(f'Read input_data = {input_data}')

                prompt = input_data.get('prompt', None)
                negative_prompt = input_data.get('negative_prompt', None)
                guidance_scale = input_data.get('guidance_scale', 3.5)
                num_inference_steps = int(input_data.get('num_inference_steps', 10))

                image = pipe(prompt=prompt, negative_prompt=negative_prompt, guidance_scale=guidance_scale, num_inference_steps=num_inference_steps).images[0]

                print(f'Saving {out_png_file}')
                image.save(out_png_file)

              except:
                allowed_errors_remaining -= 1
                traceback.print_exc()
                if 'KeyboardInterrupt' in exception_str: # We actually do want these to be fatal!
                  allowed_errors_remaining -= 999
                with open(out_txt_file, 'w') as fd:
                  fd.write(traceback.format_exc())

    except:
      allowed_errors_remaining -= 1
      traceback.print_exc()
      exception_str = traceback.format_exc()

      if 'KeyboardInterrupt' in exception_str: # We actually do want these to be fatal!
        break

    time.sleep(0.100) # Poll several times a second for new work

"#),
          c_str!("in_memory.py"),
          c_str!("in_memory"),
      )?;

      let python_entry_fn: Py<PyAny> = python_module.getattr("main").map_err(oliana_lib::eloc!())?.into();

      python_entry_fn.call1(py, (env_var_work_dir, INFERENCE_TYPE.to_string(), ) ).map_err(oliana_lib::eloc!())?;

      Ok(())
  })
}


