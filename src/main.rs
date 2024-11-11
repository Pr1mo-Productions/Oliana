use kalosm::language::*;

use futures_util::StreamExt;
use kalosm_vision::{Wuerstchen, WuerstchenInferenceSettings};

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

    // First create a model. Chat models tend to work best with structured generation
    let model = Llama::phi_3().await.unwrap();
    // Then create a task with the parser as constraints
    let task = Task::builder_for::<[Character; 10]>("You generate realistic JSON placeholders for characters")
        .build();
    // Finally, run the task
    let mut stream = task.run("Create a list of random characters", &model);
    stream.to_std_out().await.unwrap();
    let character = stream.await.unwrap();
    println!("{character:?}");

    // Image stuff
    let model = Wuerstchen::builder().build().await.unwrap();
    let settings = WuerstchenInferenceSettings::new(
        "a cute cat with a hat in a room covered with fur with incredible detail",
    );

    if let Ok(mut images) = model.run(settings) {
        while let Some(image) = images.next().await {
            if let Some(buf) = image.generated_image() {
                buf.save(&format!("{}.png", image.sample_num())).unwrap();
            }
        }
    }

}




