
#![allow(dead_code)]

use crate::*;


// first string is type of prompt, second string is prompt text; TODO possible args to add configs that go to oliana_text and oliana_images
#[derive(Debug, bevy::ecs::event::Event)]
pub struct PromptToAI(pub String, pub String);

// first string is type of prompt, second string is prompt reply. if "text" second string is simply the string, if "image" the second string is a file path to a .png.
#[derive(Debug, bevy::ecs::event::Event)]
pub struct ResponseFromAI(pub String, pub String);


// A unit struct to help identify the Ollama Reply UI component, since there may be many Text components
#[derive(Component)]
pub struct LLM_ReplyText;

// A unit struct to help identify the server URL text in the upper-right of the UI
#[derive(Component)]
pub struct Server_URL;

// A unit struct to tag+identify the sprite behind text & rest of UI
#[derive(Component)]
pub struct Background_Image;

// A unit struct to tag+identify the sprite in front of the background who is speaking
#[derive(Component)]
pub struct Foreground_Character;



