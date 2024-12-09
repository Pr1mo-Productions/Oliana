
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



async fn main_async() -> Result<(), Box<dyn std::error::Error>> {
    /*let _sdv1_4 = oliana_lib::files::existinate(
        "./SDv1-4.mpk",
        "https://huggingface.co/Gadersd/Stable-Diffusion-Burn/resolve/main/SDv1-4.mpk"
    ).await?;
    */
  use tch::nn::Module;
  use tch::Kind;

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
  // let bpe_simple_vocab_16e6_txt_path_s = bpe_simple_vocab_16e6_txt_path.to_string_lossy();

  // tch::maybe_init_cuda(); // No longer exists in 0.18+
  tch::Cuda::cudnn_set_benchmark(false); // Doesn't work -_- https://github.com/LaurentMazare/diffusers-rs/issues/16#issuecomment-1376939427

  eprintln!("Cuda available: {}", tch::Cuda::is_available());
  eprintln!("Cudnn available: {}", tch::Cuda::cudnn_is_available());
  eprintln!("Cuda num devices: {}", tch::Cuda::device_count());

  if !tch::Cuda::is_available() {
    eprintln!("Refusing to run w/o CUDA! (we _can_, it's just so slow as to be worth investigating the HW/driver issues first)");
    return Ok(());
  }

  // We assume the .safetensors above are all Floats (32 bit numbers) but modern tch's pipeline wants Half values.
  // We perform a conversion here & safe off the result, changing the file loaded below.
  /*
  let local_clip_v2_1_tensors = tch::Tensor::read_safetensors(&local_clip_v2_1_path)?;
  let local_unet_v2_1_tensors = tch::Tensor::read_safetensors(&local_unet_v2_1_path)?;
  let local_vae_v2_1_tensors = tch::Tensor::read_safetensors(&local_vae_v2_1_path)?;

  for (name, tensor) in local_clip_v2_1_tensors.iter() {
      println!("local_clip_v2_1_tensors: {name} {tensor:?}")
  }
  for (name, tensor) in local_unet_v2_1_tensors.iter() {
      println!("local_unet_v2_1_tensors: {name} {tensor:?}")
  }
  for (name, tensor) in local_vae_v2_1_tensors.iter() {
      println!("local_vae_v2_1_tensors: {name} {tensor:?}")
  }
  */

  let n_steps: usize = 24;
  let num_samples: i64 = 1;
  let guidance_scale: f64 = 7.5;
  let (width, height) = (512, 512);
  let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.subsec_nanos() as i64;

  let sd_config = diffusers::pipelines::stable_diffusion::StableDiffusionConfig::v2_1(None /* attn size */, Some(width), Some(height));

  let device_setup = diffusers::utils::DeviceSetup::new(vec![/*"vae".into(), "clip".into(), "unet".into()*/]);
  let clip_device = device_setup.get("clip");
  let vae_device = device_setup.get("vae");
  let unet_device = device_setup.get("unet");
  let scheduler = sd_config.build_scheduler(n_steps);

  let tokenizer = diffusers::transformers::clip::Tokenizer::create(bpe_simple_vocab_16e6_txt_path, &sd_config.clip).map_err(oliana_lib::eloc!())?;
  println!("Running with prompt \"{prompt_txt}\".");
  let tokens = tokenizer.encode(&prompt_txt)?;
  let tokens: Vec<i64> = tokens.into_iter().map(|x| x as i64).collect();
  let tokens = tch::Tensor::from_slice(&tokens).view((1, -1)).to(clip_device);
  let uncond_tokens = tokenizer.encode("")?;
  let uncond_tokens: Vec<i64> = uncond_tokens.into_iter().map(|x| x as i64).collect();
  let uncond_tokens = tch::Tensor::from_slice(&uncond_tokens).view((1, -1)).to(clip_device);

  let no_grad_guard = tch::no_grad_guard();

  println!("Building the Clip transformer.");
  let text_model = sd_config.build_clip_transformer(&local_clip_v2_1_path_s, clip_device)?;
  let text_embeddings = text_model.forward(&tokens);
  let uncond_embeddings = text_model.forward(&uncond_tokens);
  let text_embeddings = tch::Tensor::cat(&[uncond_embeddings, text_embeddings], 0).to(unet_device);

  println!("Building the autoencoder.");
  let vae = sd_config.build_vae(&local_vae_v2_1_path_s, vae_device).map_err(oliana_lib::eloc!())?;
  println!("Building the unet.");
  let unet = sd_config.build_unet(&local_unet_v2_1_path_s, unet_device, 4).map_err(oliana_lib::eloc!())?;

  let bsize = 1;
  for idx in 0..num_samples {
      tch::manual_seed(seed + idx);
      let mut latents = tch::Tensor::randn(
          [bsize, 4, sd_config.height / 8, sd_config.width / 8],
          (Kind::Float, unet_device),
      );

      // scale the initial noise by the standard deviation required by the scheduler
      latents *= scheduler.init_noise_sigma();

      for (timestep_index, &timestep) in scheduler.timesteps().iter().enumerate() {
          println!("Timestep {timestep_index}/{n_steps}");
          let latent_model_input = tch::Tensor::cat(&[&latents, &latents], 0);

          let latent_model_input = scheduler.scale_model_input(latent_model_input, timestep);
          let noise_pred = unet.forward(&latent_model_input, timestep as f64, &text_embeddings);
          let noise_pred = noise_pred.chunk(2, 0);
          let (noise_pred_uncond, noise_pred_text) = (&noise_pred[0], &noise_pred[1]);
          let noise_pred =
              noise_pred_uncond + (noise_pred_text - noise_pred_uncond) * guidance_scale;
          latents = scheduler.step(&noise_pred, timestep, &latents);

          /*if args.intermediary_images {
              let latents = latents.to(vae_device);
              let image = vae.decode(&(&latents / 0.18215));
              let image = (image / 2 + 0.5).clamp(0., 1.).to_device(tch::Device::Cpu);
              let image = (image * 255.).to_kind(Kind::Uint8);
              let final_image =
                  output_filename(&final_image, idx + 1, num_samples, Some(timestep_index + 1));
              tch::vision::image::save(&image, final_image)?;
          }*/
      }

      println!("Generating the final image for sample {}/{}.", idx + 1, num_samples);
      let latents = latents.to(vae_device);
      let image = vae.decode(&(&latents / 0.18215));
      let image = (image / 2 + 0.5).clamp(0., 1.).to_device(tch::Device::Cpu);
      let image = (image * 255.).to_kind(Kind::Uint8);
      tch::vision::image::save(&image, out_file_path).map_err(oliana_lib::eloc!())?;
  }
  drop(no_grad_guard);

  Ok(())
}




