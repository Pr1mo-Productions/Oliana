// Guess who doesn't care right now?
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(non_camel_case_types)]
#![allow(unreachable_code)]

use clap::Parser;

mod utils;  // src/utils.rs
mod cli;    // src/cli/mod.rs
mod gui;    // src/gui/mod.rs
mod ai;     // src/ai/mod.rs

// Main simply reads command-line arguments,
// builds a tokio async runtime, and passes state to that.
// If the tokio runtime returns an error we report it & exit w/ 1.
fn main() -> Result<(), Box<dyn std::error::Error>>  {
  let cli_args = cli::Args::parse();

  let rt  = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(std::cmp::max(2, num_cpus::get_physical())) // Use all host cores, unless single-cored in which case pretend to have 2
    .thread_stack_size(8 * 1024 * 1024)
    .enable_time()
    .enable_io()
    .build()?;

  rt.block_on(async {
    if let Err(e) = main_async(&cli_args).await {
      eprintln!("[ main_async ] {}", e);
      std::process::exit(1);
    }
  });

  Ok(())
}


// main_async should hold all of our top-level watchdog logic; fundamentally it'll be
// a manager of the GUI, any hardware access we need, and will need a little logic to be able to re-start
// subroutines if they return Err().
async fn main_async(cli_args: &cli::Args) -> Result<(), Box<dyn std::error::Error>>  {

  if cli_args.list_connected_hardware  {
    eprintln!("= = = = Compute Devices = = = =");
    let names = ai::get_compute_device_names().await?;
    for name in &names {
        eprintln!(" - {name}");
    }
  }

  if let Some(test_llm_prompt) = &cli_args.test_llm_prompt {
    eprintln!("Prompt: {test_llm_prompt}");
    let ai_response_txt = ai::run_oneshot_llm_prompt(&test_llm_prompt).await?;
    eprintln!("Response: {ai_response_txt}");
  }

  if let Some(test_image_prompt) = &cli_args.test_image_prompt {
    eprintln!("Prompt: {test_image_prompt}");
    let out_file_path = ai::run_oneshot_ai_img_prompt(&test_image_prompt, "out.png").await?;
    eprintln!("Open the output AI-generated file {out_file_path}");
  }


  // Exit if the test tools were invoked
  if cli_args.test_llm_prompt.is_some() || cli_args.test_image_prompt.is_some() || cli_args.list_connected_hardware {
    return Ok(());
  }


  // Regular game logic

  gui::open_gui_window().await?;


  Ok(())
}
