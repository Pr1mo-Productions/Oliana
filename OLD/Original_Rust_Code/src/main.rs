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
  let mut cli_args = cli::Args::parse();
  cli_args.update_from_env();

  // Silence the logs of some of our dependencies
  if cli_args.verbose < 2 {
    let mut log_builder = env_logger::Builder::new();
    log_builder
        .filter(None, log::LevelFilter::Off) // log level for your module
        .filter_module("reqwest",   log::LevelFilter::Off)  // log level for reqwest
        .filter_module("ollama_rs", log::LevelFilter::Off)  // log level for ollama
        .filter_module("ollama-rs", log::LevelFilter::Off)  // log level for ollama
        .filter_module("ollama",    log::LevelFilter::Off)  // log level for ollama
        .init();
  }

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
    let names = ai::get_compute_device_names(cli_args).await?;
    for name in &names {
        eprintln!(" - {name}");
    }
  }

  if let Some(test_llm_prompt) = &cli_args.test_llm_prompt {
    eprintln!("Prompt: {test_llm_prompt}");
    let ai_response_txt = ai::run_oneshot_llm_prompt(cli_args, &test_llm_prompt).await?;
    eprintln!("Response: {ai_response_txt}");
  }

  if let Some(test_image_prompt) = &cli_args.test_image_prompt {
    eprintln!("Prompt: {test_image_prompt}");
    let out_file_path = ai::run_oneshot_ai_img_prompt(cli_args, &test_image_prompt, "out.png").await?;
    eprintln!("Open the output AI-generated file {out_file_path}");
  }


  // Exit if the test tools were invoked
  if cli_args.test_llm_prompt.is_some() || cli_args.test_image_prompt.is_some() || cli_args.list_connected_hardware {
    return Ok(());
  }

  // Beginning of game startup in the gui module
  gui::open_gui_window(cli_args).await?;


  Ok(())
}
