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

  if cli_args.verbose > 0 {
    eprintln!("= = = = Compute Devices = = = =");
    let names = ai::get_openvino_compute_device_names()?;
    for name in &names {
        eprintln!(" - {name}");
    }
    tokio::time::sleep(std::time::Duration::from_millis(5 * 1000)).await;
  }

  gui::open_gui_window().await?;


  Ok(())
}
