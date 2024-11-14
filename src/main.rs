
// Turn off compiler warnings we don't care about while in R&D phase
#![allow(dead_code)]

use kalosm::language::*;

use kalosm::vision::{Wuerstchen, WuerstchenInferenceSettings};

mod utils; // src/utils.rs

/// A fictional character
#[derive(Parse, Schema, Clone, Debug)]
struct Character {
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

#[tokio::main]
async fn main() {
    let main_start = std::time::Instant::now();

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

    // Image stuff
    let image_start = std::time::Instant::now();

    /*
    let model = Wuerstchen::builder()
        //.with_flash_attn(true) // reduce GPU vram required - requires kalosm to expise a feature flag!
        .build().await.unwrap();
    let settings = WuerstchenInferenceSettings::new(
        "a cute cat with a hat in a room covered with fur with incredible detail",
    )
    .with_prior_steps(4) // todo increase
    .with_denoiser_steps(4)
    .with_sample_count(12);

    if let Ok(mut images) = model.run(settings) {
        while let Some(image) = images.next().await {
            if let Some(buf) = image.generated_image() {
                let file = format!("{}.png", image.sample_num());
                buf.save(&file).unwrap();
                eprintln!("Saved {}", &file);
            }
        }
    }*/

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

    let prompt = "A cowboy exploring a computer cave";
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

    println!("Total Time: {}", utils::duration_to_display_str(&(image_end - main_start)));
    println!("LLM Generation Time: {}", utils::duration_to_display_str(&(llm_end - llm_start)));
    println!("Image Generation Time: {}", utils::duration_to_display_str(&(image_end - image_start)));



}





use stablediffusion::{
    model::stablediffusion::{load::load_stable_diffusion, *},
    tokenizer::SimpleTokenizer,
};

use burn::{
    config::Config,
    module::{Module, Param},
    nn,
    tensor::{backend::Backend, Tensor},
};

use burn_tch::{LibTorch, LibTorchDevice};

use std::env;
use std::io;
use std::process;

use burn::record::{self, NamedMpkFileRecorder, FullPrecisionSettings, Recorder};


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
