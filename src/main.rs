
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
    let model = Wuerstchen::builder()
        .with_flash_attn(true) // reduce GPU vram required
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
                buf.save(&format!("{}.png", image.sample_num())).unwrap();
            }
        }
    }
    let image_end = std::time::Instant::now();

    println!("Total Time: {}", utils::duration_to_display_str(&(image_end - main_start)));
    println!("LLM Generation Time: {}", utils::duration_to_display_str(&(llm_start - llm_end)));
    println!("Image Generation Time: {}", utils::duration_to_display_str(&(image_start - image_end)));



}





