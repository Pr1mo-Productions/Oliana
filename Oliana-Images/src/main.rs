
#![allow(unused_variables)]


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
use pyo3::types::IntoPyDict;
use pyo3::ffi::c_str;

async fn main_async() -> Result<(), Box<dyn std::error::Error>> {

  let out_file_path = "out.png";
  let prompt_txt = "Photograph of a cowboy riding over the moon at night";

  // First download all the models
  let local_clip_v2_1 = oliana_lib::files::get_cache_file("rust-stable-diffusion-v2-1_clip_v2.1.safetensors").await.map_err(oliana_lib::eloc!())?;
  let local_clip_v2_1_path = oliana_lib::files::existinate(
    &local_clip_v2_1, "https://huggingface.co/lmz/rust-stable-diffusion-v2-1/resolve/main/weights/clip_v2.1.safetensors"
  ).await.map_err(oliana_lib::eloc!())?;
  let local_clip_v2_1_path_s = local_clip_v2_1_path.to_string_lossy();

  let local_vae_v2_1 = oliana_lib::files::get_cache_file("rust-stable-diffusion-v2-1_vae_v2.1.safetensors").await.map_err(oliana_lib::eloc!())?;
  let local_vae_v2_1_path = oliana_lib::files::existinate(
    &local_vae_v2_1, "https://huggingface.co/lmz/rust-stable-diffusion-v2-1/resolve/main/weights/vae_v2.1.safetensors"
  ).await.map_err(oliana_lib::eloc!())?;
  let local_vae_v2_1_path_s = local_vae_v2_1_path.to_string_lossy();

  let local_unet_v2_1 = oliana_lib::files::get_cache_file("rust-stable-diffusion-v2-1_unet_v2.1.safetensors").await.map_err(oliana_lib::eloc!())?;
  let local_unet_v2_1_path = oliana_lib::files::existinate(
    &local_unet_v2_1, "https://huggingface.co/lmz/rust-stable-diffusion-v2-1/resolve/main/weights/unet_v2.1.safetensors"
  ).await.map_err(oliana_lib::eloc!())?;
  let local_unet_v2_1_path_s = local_unet_v2_1_path.to_string_lossy();

  let bpe_simple_vocab_16e6_txt = oliana_lib::files::get_cache_file("rust-stable-diffusion-v2-1_bpe_simple_vocab_16e6.txt").await.map_err(oliana_lib::eloc!())?;
  let bpe_simple_vocab_16e6_txt_path = oliana_lib::files::existinate(
    &bpe_simple_vocab_16e6_txt, "https://huggingface.co/lmz/rust-stable-diffusion-v2-1/raw/main/weights/bpe_simple_vocab_16e6.txt"
  ).await.map_err(oliana_lib::eloc!())?;

  let site_packages = oliana_lib::files::get_cache_file("Oliana-Images-site_packages").await.map_err(oliana_lib::eloc!())?;
  let site_packages = site_packages.to_string_lossy();
  tokio::fs::create_dir_all(&site_packages[..]).await?;

  let pythonpath = std::env::join_paths(&[
    site_packages.to_string(),
    std::env::var("PYTHONPATH").unwrap_or("".to_string()),
  ]).map_err(oliana_lib::eloc!())?;


  std::env::set_var(
    "PYTHONPATH", pythonpath
  );

  python_main(&site_packages).map_err(oliana_lib::eloc!())?;

  Ok(())
}

fn python_main(site_packages: &str) -> PyResult<()> {
  Python::with_gil(|py| {
      let sys = py.import("sys")?;
      let version: String = sys.getattr("version")?.extract()?;

      println!("Oliana-Images is using Python {version} for processing");

      let pip = py.import("pip")?;
      let pip_main: Py<PyAny> = pip.getattr("main")?.into();

      if let Err(e) = py.import("torch") {
        eprintln!("{:?}", e);
        let arg_vals = vec![
          "install".to_string(), format!("--target={site_packages}"), "torch".to_string(), "torchvision".to_string(), "torchaudio".to_string()
        ];
        let args = (arg_vals, );
        pip_main.call1(py, args)?;
      }

      let torch = py.import("torch")?;

      eprintln!("torch = {:?}", torch);



      Ok(())
  })
}


