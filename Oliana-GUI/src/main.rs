
#![allow(unused_imports, unused_mut, unused_variables, non_camel_case_types)]

use bevy::{
    core::FrameCount,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{CursorGrabMode, PresentMode, WindowLevel, WindowTheme},
};

use bevy::prelude::*;

use bevy_simple_text_input::{
    TextInputBinding, TextInputInactive, TextInputPlugin, TextInputSystem, TextInputSubmitEvent
};

use bevy_defer::AsyncCommandsExtension;

use bevy_simple_scroll_view::*;

const CLEAR_TOKEN: &'static str = "!!!CLEAR!!!";

use clap::Parser;

mod structs;
// Holds Bevy-annotated structures
mod gui_structs;
// Setup holds a single large function that constructs the Bevy UI components and links them to event handlers
mod gui_setup;
// gui_updaters contain event-based callbacks which perform per-frame rendering logic and event management such as talking to/from the AI server
mod gui_updaters;
// gui_painters contains functions which do the same as gui_updaters, but are vastly simpler and do not care about global state (such as hover UI render logic)
mod gui_painters;
// gui_oneshot_tasks contains functions that will run once and then stop
mod gui_oneshot_tasks;

lazy_static::lazy_static! {
    static ref GLOBALS: std::sync::RwLock::<structs::Globals> = std::sync::RwLock::new(structs::Globals::new());
}

fn main() -> Result<(), Box<dyn std::error::Error>>  {
  let mut cli_args = structs::Args::parse();
  cli_args.update_from_env();

  std::env::set_var("RUST_LOG", "none,oliana_gui=debug");

  let rt  = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(std::cmp::max(2, num_cpus::get_physical())) // Use all host cores, unless single-cored in which case pretend to have 2
    .thread_stack_size(8 * 1024 * 1024)
    .enable_time()
    .enable_io()
    .build()?;

  rt.block_on(async {
    if let Err(e) = main_async(&cli_args).await {
      eprintln!("[ main_async ] {}", e);
      cleanup_child_procs();
      std::process::exit(1);
    }
  });
  cleanup_child_procs();
  std::process::exit(0);
  // Ok(())
}

pub fn poll_until_exit_or_elapsed(sys: &mut sysinfo::System, pid: usize, ms_to_poll_for: isize) {
    let mut remaining_ms: isize = ms_to_poll_for;
    const POLL_MS: isize = 50;
    let pid = sysinfo::Pid::from(pid);
    while remaining_ms > 1 {
        remaining_ms -= POLL_MS;
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        if let Some(_process) = sys.process(sysinfo::Pid::from(pid)) {
            // Process is still running!
        }
        else {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(POLL_MS as u64));
    }
}

pub fn cleanup_child_procs() {
  if let Ok(mut globals_wl) = GLOBALS.write() {

    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    if let Some(ref mut server_proc) = &mut globals_wl.server_proc {
        if let Some(process) = sys.process(sysinfo::Pid::from(server_proc.id() as usize)) {
            process.kill_with(sysinfo::Signal::Term);
        }
        poll_until_exit_or_elapsed(&mut sys, server_proc.id() as usize, 1800);
        if let Some(_process) = sys.process(sysinfo::Pid::from(server_proc.id() as usize)) {
            eprintln!("[ Note ] oliana_server did not exit in 1800ms, killing...");
            if let Err(e) = server_proc.kill() {
                eprintln!("{}:{} {:?}", file!(), line!(), e);
            }
        }
        else {
            eprintln!("[ Note ] oliana_server cleanly exited with SigTerm");
        }
    }
    // We walk globals_wl.track_proc_dir for "*-pid.txt" files, read the first integer, and kill those processes as well.
    let mut potential_child_pids: Vec<usize> = vec![];
    match std::fs::read_dir(&globals_wl.track_proc_dir) {
        Ok(child_dirents) => {
            for dirent in child_dirents {
                if let Ok(dirent) = dirent {
                    let path = dirent.path();
                    let path_s = path.to_string_lossy();
                    if path_s.ends_with("-pid.txt") || path_s.ends_with("-pid.TXT") {
                        if let Ok(path_content_s) = std::fs::read_to_string(&path) {
                            let path_content_s_trimmed = path_content_s.trim();
                            if let Ok(child_pid_num) = path_content_s_trimmed.parse::<usize>() {
                                potential_child_pids.push(child_pid_num);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{}:{} {:?}", file!(), line!(), e);
        }
    }

    for child_pid_num in &potential_child_pids {
        if let Some(process) = sys.process(sysinfo::Pid::from(*child_pid_num)) {
            eprintln!("[ Note ] Sending SigTerm to {:?} ({})", process.exe(), child_pid_num );
            process.kill_with(sysinfo::Signal::Term);
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(1800));

    for child_pid_num in &potential_child_pids {
        if let Some(process) = sys.process(sysinfo::Pid::from(*child_pid_num)) {
            eprintln!("[ Note ] Sending SigKill to {:?} ({})", process.exe(), child_pid_num );
            process.kill_with(sysinfo::Signal::Kill);
        }
    }

  }
}

pub async fn main_async(cli_args: &structs::Args) -> Result<(), Box<dyn std::error::Error>> {

  if let Ok(mut globals_wl) = GLOBALS.write() {
    if let Err(e) = globals_wl.initialize() {
        eprintln!("{}:{} {}", file!(), line!(), e);
    }
  }
  tokio::task::spawn(async { // We want a runtime handle, but we also do not want to pin to this async thread as it will be used 100% by the Bevy engine below.
    if let Ok(mut globals_wl) = GLOBALS.write() {
        globals_wl.tokio_rt = Some( tokio::runtime::Handle::current() );
    }
  });

  App::new()
    .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Oliana - The Game".into(),
                    name: Some("Oliana - The Game".into()),
                    resolution: (500., 300.).into(),
                    present_mode: PresentMode::AutoVsync,
                    window_theme: Some(WindowTheme::Dark),
                    enabled_buttons: bevy::window::EnabledButtons {
                        // maximize: false,
                        ..Default::default()
                    },
                    // This will spawn an invisible window
                    // The window will be made visible in the make_visible() system after 3 frames.
                    // This is useful when you want to avoid the white window that shows up before the GPU is ready to render the app.
                    visible: false,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::render::RenderPlugin { // This configuration uses the builtin GPU before a dgpu; Jeff saw some wierd crashes while running lots of sw all reaching for the dgpu, so this is here as a small reliability improver.
                render_creation: bevy::render::settings::RenderCreation::Automatic(
                    bevy::render::settings::WgpuSettings {
                        power_preference: bevy::render::settings::PowerPreference::LowPower, // TODO some boxes will crash if LowPower isn't available!
                        ..default()
                    }
                ),
                ..default()
            }))
    .add_plugins(bevy_defer::AsyncPlugin::default_settings())
    .add_plugins(TextInputPlugin)
    .add_plugins(ScrollViewPlugin)

    .add_event::<gui_structs::PromptToAI>()
    .add_event::<gui_structs::ResponseFromAI>()

    .insert_resource((*cli_args).clone()) // Accept a Ref<crate::cli::Args> in your system's function to read cli args in the UI

    .add_systems(Update, gui_updaters::render_server_url_in_use.run_if(bevy::time::common_conditions::on_timer(bevy::utils::Duration::from_millis(400))) )
    .add_systems(Update, gui_updaters::drain_global_events_to_bevy.run_if(bevy::time::common_conditions::on_timer(bevy::utils::Duration::from_millis(40))) )

    .add_systems(Update, gui_updaters::make_visible )
    .add_systems(Startup, (gui_setup::gui_setup, gui_oneshot_tasks::determine_if_we_have_local_gpu) )
    .add_systems(Update, gui_painters::focus.before(TextInputSystem))
    .add_systems(Update, gui_updaters::text_listener.after(TextInputSystem))
    .add_systems(Update, gui_updaters::read_ai_response_events)
    .add_systems(Update, gui_updaters::read_ai_prompt_events)
    .add_systems(Update, gui_painters::reset_scroll) // TODO move this down/make it accessible someplace

   .run();

   Ok(())
}
