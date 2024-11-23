


pub fn duration_to_display_str(d: &std::time::Duration) -> String {
  let total_millis = d.as_millis();
  let ms = total_millis % 1000;
  let s = (total_millis / 1000) % 60;
  let m = (total_millis / (1000 * 60)) % 60;
  let h = total_millis / (1000 * 60 * 60) /* % 24 */;
  if h > 0 {
    format!("{:0>2}h {:0>2}m {:0>2}s {:0>3}ms", h, m, s, ms)
  }
  else if m > 0 {
    format!("{:0>2}m {:0>2}s {:0>3}ms", m, s, ms)
  }
  else if s > 0 {
    format!("{:0>2}s {:0>3}ms", s, ms)
  }
  else {
    format!("{:0>3}ms", ms)
  }
}



use stablediffusion::{
    model::stablediffusion::*,
    tokenizer::SimpleTokenizer,
};

use burn::{
    module::Module,
    tensor::backend::Backend,
};



use burn::record::{self, NamedMpkFileRecorder, FullPrecisionSettings, Recorder};


/// returns file path to generated image
pub async fn use_stable_diffusion_to_gen_a_photo(prompt: &str) -> String {
  let image_start = std::time::Instant::now();

  if !std::path::Path::new("SDv1-4.mpk").exists() {
      // Download it!
      let mut downloader = downloader::Downloader::builder()
          .download_folder(std::path::Path::new("."))
          .parallel_requests(2)
          .build()
          .unwrap();

      let dl = downloader::Download::new("https://huggingface.co/Gadersd/Stable-Diffusion-Burn/resolve/main/SDv1-4.mpk");

      let result = downloader.async_download(&[dl]).await.unwrap();

      for r in result {
          match r {
              Err(e) => println!("Error: {}", e.to_string()),
              Ok(s) => println!("Success: {}", &s),
          };
      }
  }

  if !std::path::Path::new("bpe_simple_vocab_16e6.txt").exists() {
      // Download it!
      let mut downloader = downloader::Downloader::builder()
          .download_folder(std::path::Path::new("."))
          .parallel_requests(2)
          .build()
          .unwrap();

      let dl = downloader::Download::new("https://raw.githubusercontent.com/Gadersd/stable-diffusion-burn/refs/heads/main/bpe_simple_vocab_16e6.txt");

      let result = downloader.async_download(&[dl]).await.unwrap();

      for r in result {
          match r {
              Err(e) => println!("Error: {}", e.to_string()),
              Ok(s) => println!("Success: {}", &s),
          };
      }
  }

  use burn_tch::{LibTorch, LibTorchDevice};


  const SD_FILE: &'static str = "SDv1-4.mpk";

  // type Backend = LibTorch<f32>;

  //let device = LibTorchDevice::Cuda(0);
  let device = LibTorchDevice::Cpu;
  //let device = LibTorchDevice::Mps;

  println!("Loading tokenizer...");
  let tokenizer = SimpleTokenizer::new().unwrap(); // requires the file "bpe_simple_vocab_16e6.txt"
  println!("Loading model...");
  let sd: StableDiffusion<LibTorch<f32>> = load_stable_diffusion_model_file(SD_FILE, &device).expect("Could not load sd model file");

  let unconditional_guidance_scale: f64 = 6.5;
  let n_steps: usize = 16;
  let output_image_prefix = "out-";

  let unconditional_context = sd.unconditional_context(&tokenizer);
  let context = sd.context(&tokenizer, prompt).unsqueeze::<3>(); //.repeat(0, 2); // generate 2 samples

  println!("Sampling image...");
  let images = sd.sample_image(
      context,
      unconditional_context,
      unconditional_guidance_scale,
      n_steps,
  );

  if let Err(e) = save_images(&images, output_image_prefix, 512, 512) {
      eprintln!("{:?}", e);
  }

  let image_end = std::time::Instant::now();
  println!("Image Generation Time: {}", duration_to_display_str(&(image_end - image_start)));
  return format!("{}0.png", output_image_prefix); // assuming we got at least 1 back
}




fn load_stable_diffusion_model_file<B: Backend>(
    filename: &str,
    device: &B::Device,
) -> Result<StableDiffusion<B>, record::RecorderError> {
    NamedMpkFileRecorder::<FullPrecisionSettings>::new()
        .load(filename.into(), device)
        .map(|record| StableDiffusionConfig::new().init(device).load_record(record))
}

use image;
use image::{ColorType::Rgb8, ImageResult};

fn save_images(images: &Vec<Vec<u8>>, basepath: &str, width: u32, height: u32) -> ImageResult<()> {
    for (index, img_data) in images.iter().enumerate() {
        let path = format!("{}{}.png", basepath, index);
        image::save_buffer(path, &img_data[..], width, height, Rgb8)?;
    }

    Ok(())
}


use kalosm::language::*;


/// A fictional character
#[derive(Parse, Schema, Clone, Debug)]
pub struct Character {
    /// The name of the character
    #[parse(pattern = "[A-Z][a-z]{2,10} [A-Z][a-z]{2,10}")]
    name: String,
    /// The age of the character
    #[parse(range = 1..=100)]
    age: u8,
    /// A description of the character
    #[parse(pattern = "[A-Za-z ]{40,200}")]
    description: String,
}


pub async fn gen_an_llm_powered_character() -> Character {
  let llm_start = std::time::Instant::now();
    // First create a model. Chat models tend to work best with structured generation
    let model = Llama::phi_3().await.unwrap();
    // Then create a task with the parser as constraints
    let task = Task::builder_for::<[Character; 2]>("You generate realistic JSON placeholders for characters")
        .build();
    // Finally, run the task
    let mut stream = task.run("Create a list of random characters", &model);
    stream.to_std_out().await.unwrap();
    let character = stream.await.unwrap();
    let llm_end = std::time::Instant::now();
    println!("{character:?}");

    println!("LLM Gen time: {}", duration_to_display_str(&(llm_end - llm_start)));

    return character[0].clone();
}




