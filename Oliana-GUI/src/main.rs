
#![allow(unused_imports, unused_mut, unused_variables, non_camel_case_types)]

use bevy::{
    core::FrameCount,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{CursorGrabMode, PresentMode, WindowLevel, WindowTheme},
};

use bevy::prelude::*;

use bevy_simple_text_input::{
    TextInputBundle, TextInputInactive, TextInputPlugin, TextInputSystem, TextInputSubmitEvent
};

use bevy_defer::AsyncCommandsExtension;

use bevy_simple_scroll_view::*;

const BORDER_COLOR_ACTIVE: Color = Color::srgb(0.75, 0.52, 0.99);
const BORDER_COLOR_INACTIVE: Color = Color::srgb(0.25, 0.25, 0.25);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const BACKGROUND_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);
const LLM_OUTPUT_BACKGROUND_COLOR: Color = Color::srgb(0.18, 0.12, 0.18); // 138,65,138

const CLEAR_TOKEN: &'static str = "!!!CLEAR!!!";

use clap::Parser;
mod structs;

lazy_static::lazy_static! {
    static ref GLOBALS: std::sync::RwLock::<structs::Globals> = std::sync::RwLock::new(structs::Globals::new());
}

fn main() -> Result<(), Box<dyn std::error::Error>>  {
  let mut cli_args = structs::Args::parse();
  cli_args.update_from_env();

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
  if let Ok(mut globals_wl) = GLOBALS.try_write() {

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

  if let Ok(mut globals_wl) = GLOBALS.try_write() {
    if let Err(e) = globals_wl.initialize() {
        eprintln!("{}:{} {}", file!(), line!(), e);
    }
  }
  tokio::task::spawn(async { // We want a runtime handle, but we also do not want to pin to this async thread as it will be used 100% by the Bevy engine below.
    if let Ok(mut globals_wl) = GLOBALS.try_write() {
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
                        power_preference: bevy::render::settings::PowerPreference::LowPower,
                        ..default()
                    }
                ),
                ..default()
            }))
    .add_plugins(bevy_defer::AsyncPlugin::default_settings())
    .add_plugins(TextInputPlugin)
    .add_plugins(ScrollViewPlugin)

    .add_event::<PromptToAI>()
    .add_event::<ResponseFromAI>()

    .insert_resource((*cli_args).clone()) // Accept a Ref<crate::cli::Args> in your system's function to read cli args in the UI
    //.insert_resource(OllamaResource::default()) // Accept a Ref<crate::gui::OllamaResource> in your system's function to touch the Ollama stuff

    .add_systems(Update, (make_visible, render_server_url_in_use) )
    .add_systems(Startup, (setup, determine_if_we_have_local_gpu) )
    .add_systems(Update, focus.before(TextInputSystem))
    .add_systems(Update, text_listener.after(TextInputSystem))
    .add_systems(Update, read_ollama_response_events)
    .add_systems(Update, read_ollama_prompt_events)
    .add_systems(Update, reset_scroll) // TODO move this down/make it accessible someplace

   .run();

   Ok(())
}

fn make_visible(mut window: Query<&mut Window>, frames: Res<FrameCount>) {
    // The delay may be different for your app or system.
    if frames.0 == 3 {
        // At this point the gpu is ready to show the app so we can make the window visible.
        // Alternatively, you could toggle the visibility in Startup.
        // It will work, but it will have one white frame before it starts rendering
        window.single_mut().visible = true;
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    const STATUS_BAR_HEIGHT: f32 = 60.0;

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::End, // End here means "Bottom"
                    justify_content: JustifyContent::Start, // Start here means "Left"
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                ..default()
            },
            // Make this container node bundle to be Interactive so that clicking on it removes
            // focus from the text input.
            Interaction::None,
        ))
        .with_children(|parent| {
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(80.0),
                        border: UiRect::all(Val::Px(2.0)),
                        padding: UiRect::all(Val::Px(2.0)),
                        margin: UiRect {
                            left: Val::Px(2.0),
                            top: Val::Px(2.0),
                            right: Val::Px(2.0),
                            bottom: Val::Px(STATUS_BAR_HEIGHT + 2.0),
                        },
                        ..default()
                    },
                    border_color: BORDER_COLOR_INACTIVE.into(),
                    background_color: BACKGROUND_COLOR.into(),
                    // Prevent clicks on the input from also bubbling down to the container
                    // behind it
                    focus_policy: bevy::ui::FocusPolicy::Block,
                    ..default()
                },
                TextInputBundle::default()
                    .with_text_style(TextStyle {
                        font_size: 32.0,
                        color: TEXT_COLOR,
                        ..default()
                    })
                    //.with_placeholder("Click to Type Text", None)
                    .with_inactive(true),
            ));

            parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        right: Val::Px(4.0),
                        bottom: Val::Px(2.0),
                        height: Val::Px(STATUS_BAR_HEIGHT),
                        align_items: AlignItems::End, // Start here means "Top"
                        justify_content: JustifyContent::Start, // Start here means "Left"
                        padding: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    // z_index: ZIndex::Global(1000), // This ensures the text floats _above_ the LLM response text.
                    ..default()
                },
            )).with_children(|node_bundle| {
                node_bundle.spawn((
                    TextBundle::from_section(
                        // Accepts a `String` or any type that converts into a `String`, such as `&str`
                        "127.0.0.1:9050",
                        TextStyle {
                            // This font is loaded and will be used instead of the default font.
                            // font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 14.0,
                            color: TEXT_COLOR,
                            ..default()
                        },
                    ) // Set the justification of the Text
                    .with_text_justify(JustifyText::Left)
                    // Set the style of the TextBundle itself.
                    .with_style(Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(1.0),
                        top: Val::Px(1.0),
                        right: Val::Px(1.0),
                        bottom: Val::Px(1.0),
                        //width: Val::Px(400.0),
                        margin: UiRect::all(Val::Px(1.0)),
                        //border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(1.0)),
                        ..default()
                    }),
                    Server_URL,
                ));
            });

        });

    commands.spawn(( // TODO move me upstairs?
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(4.0),
                left: Val::Px(4.0),
                right: Val::Px(4.0),
                bottom: Val::Px(STATUS_BAR_HEIGHT + 56.0),
                align_items: AlignItems::Start, // Start here means "Top"
                justify_content: JustifyContent::Start, // Start here means "Left"
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            border_color: BORDER_COLOR_INACTIVE.into(),
            background_color: LLM_OUTPUT_BACKGROUND_COLOR.into(),
            ..default()
        },
        ScrollView::default(),
    ))
    .with_children(|scroll_area| {
        scroll_area.spawn((
            TextBundle::from_section(
                // Accepts a `String` or any type that converts into a `String`, such as `&str`
                "hello\nbevy!",
                TextStyle {
                    // This font is loaded and will be used instead of the default font.
                    // font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    ..default()
                },
            ) // Set the justification of the Text
            .with_text_justify(JustifyText::Left)
            // Set the style of the TextBundle itself.
            .with_style(Style {
                align_self: AlignSelf::Stretch,
                /*position_type: PositionType::Absolute,
                top: Val::Px(4.0),
                left: Val::Px(4.0),
                right: Val::Px(4.0),
                bottom: Val::Px(56.0),*/
                // min_height: Val::Px(900.0),
                margin: UiRect::all(Val::Px(4.0)),
                //border: UiRect::all(Val::Px(5.0)),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            }),
            LLM_ReplyText,
            ScrollableContent::default(),
        ));
    });


}

fn render_server_url_in_use(mut window: Query<&mut Window>, frames: Res<FrameCount>, mut query: Query<&mut Text, With<Server_URL>>) {
    if frames.0 % 24 == 0 {
        let server_pcie_devices = ask_server_for_pci_devices(); // ask_server_for_pci_devices needs try_write() and doing that within a try_read() always fails
        if let Ok(globals_rl) = GLOBALS.try_read() {
            let server_url: String = globals_rl.server_url.clone();
            let mut server_txt = format!("{}\n", &server_url);
            let num_devices = server_pcie_devices.len();
            for (i, device_name) in server_pcie_devices.iter().enumerate() {
                server_txt.push_str(&format!("({}) {}", i, &device_name));
                if i != num_devices-1 {
                    server_txt.push_str(" // ");
                }
            }
            for mut text in &mut query { // Append to existing content in support of a streaming design.
                text.sections[0].value = server_txt.clone();
            }
        }
    }
}

fn ask_server_for_pci_devices() -> Vec<String> {
    let mut pcie_devices = vec![];
    let mut server_url = String::new();
    if let Ok(globals_rl) = GLOBALS.try_read() {
        server_url.push_str(&globals_rl.server_url);
        if let Some(cached_pcie_devices) = globals_rl.server_pcie_devices.get(&globals_rl.server_url) {
            if cached_pcie_devices.len() > 0 {
                for d in cached_pcie_devices.iter() {
                    pcie_devices.push(d.clone());
                }
                return pcie_devices;
            }
        }
    }
    if server_url.len() > 0 {
        if let Ok(mut globals_wl) = GLOBALS.try_write() {
            if let Some(tokio_rt) = &globals_wl.tokio_rt {
                tokio_rt.block_on(async {
                    if let Err(e) = ask_server_for_pci_devices_async(&server_url, &mut pcie_devices).await {
                        eprintln!("{}:{} {:?}", file!(), line!(), e);
                    }
                });
            }
            else {
                eprintln!("{}:{} globals_wl.tokio_rt is None!", file!(), line!() );
            }
            if pcie_devices.len() > 0 {
                let server_url_clone = globals_wl.server_url.clone();
                globals_wl.server_pcie_devices.insert(server_url_clone, pcie_devices.clone() );
            }
        }
        else {
            eprintln!("{}:{} GLOBALS.try_write() cannot be aquired!", file!(), line!() );
        }
    }
    return pcie_devices;
}

async fn ask_server_for_pci_devices_async(server_url: &str, pcie_devices: &mut Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut transport = tarpc::serde_transport::tcp::connect(server_url, tarpc::tokio_serde::formats::Bincode::default);
    transport.config_mut().max_frame_length(usize::MAX);

    let client = oliana_server_lib::OlianaClient::new(tarpc::client::Config::default(), transport.await?).spawn();

    let mut hardware_names = client.fetch_pci_hw_device_names(tarpc::context::current()).await?;

    pcie_devices.append(&mut hardware_names);

    Ok(())
}

fn focus(
    query: Query<(Entity, &Interaction), Changed<Interaction>>,
    mut text_input_query: Query<(Entity, &mut TextInputInactive, &mut BorderColor)>,
) {
    for (interaction_entity, interaction) in &query {
        if *interaction == Interaction::Pressed {
            for (entity, mut inactive, mut border_color) in &mut text_input_query {
                if entity == interaction_entity {
                    inactive.0 = false;
                    *border_color = BORDER_COLOR_ACTIVE.into();
                } else {
                    inactive.0 = true;
                    *border_color = BORDER_COLOR_INACTIVE.into();
                }
            }
        }
    }
}

fn text_listener(mut events: EventReader<TextInputSubmitEvent>, mut event_writer: EventWriter<PromptToAI>,) {
    for event in events.read() {
        info!("{:?} submitted: {}", event.entity, event.value);
        event_writer.send(PromptToAI("text".into(), event.value.clone()));
    }
}


fn read_ollama_response_events(
    mut event_reader: EventReader<ResponseFromAI>,
    mut query: Query<&mut Text, With<LLM_ReplyText>>
) {
    for ev in event_reader.read() {
        eprintln!("Event {:?} recieved!", ev);
        let renderable_string = ev.0.to_string();
        let renderable_string = renderable_string.replace("—", "-"); // Language models can produce hard-to-render glyphs which we manually remove here.
        if ev.0 == CLEAR_TOKEN {
            // Clear the screen
            for mut text in &mut query { // We'll only ever have 1 section of text rendered
                text.sections[0].value = String::new();
            }
        }
        else {
            for mut text in &mut query { // Append to existing content in support of a streaming design.
                text.sections[0].value = format!("{}{}", text.sections[0].value, renderable_string.to_string());
            }
        }
    }
}

fn read_ollama_prompt_events(
    mut commands: Commands,
    mut event_reader: EventReader<PromptToAI>,
    // mut event_writer: EventWriter<ResponseFromAI>,
) {

    for ev in event_reader.read() {
        let ev_txt = ev.0.to_string();
        eprintln!("Passing this prompt to Ollama: {:?}", ev.0);

        commands.spawn_task(|| async move {

            let r = bevy_defer::access::AsyncWorld.send_event(ResponseFromAI("text".into(), CLEAR_TOKEN.to_string() ));
            if let Err(e) = r {
                eprintln!("[ read_ollama_prompt_events ] {:?}", e);
            }

            /*match ollama_resource_readlock.generate_stream(ollama_rs::generation::completion::request::GenerationRequest::new(closure_owned_desired_model_name, ev_txt)).await {
                Ok(mut reply_stream) => {
                    while let Some(Ok(several_responses)) = reply_stream.next().await {
                        for response in several_responses.iter() {
                            let r = bevy_defer::access::AsyncWorld.send_event(ResponseFromAI("text".into(), response.response.to_string() ));
                            if let Err(e) = r {
                                eprintln!("[ read_ollama_prompt_events ] {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("e = {:?}", e);
                    let r = bevy_defer::access::AsyncWorld.send_event(ResponseFromAI("text".into(), format!("{:?}", e)));
                    if let Err(e) = r {
                        eprintln!("[ read_ollama_prompt_events ] {:?}", e);
                    }
                }
            }*/

            Ok(())
        });
    }
}

fn poll_for_write_lock<T>(arc_rwlock: &std::sync::Arc::<std::sync::RwLock::<T>>, num_retries: usize, retry_delay_s: u64) -> std::sync::RwLockWriteGuard<T> {
    let mut remaining_polls = num_retries;
    loop {
        remaining_polls -= 1;
        match std::sync::RwLock::write(&arc_rwlock) {
            Ok(rwlock) => {
                return rwlock;
            }
            Err(e) => {
                eprintln!("[ poll_for_write_lock ] {:?}", e);
                std::thread::sleep(std::time::Duration::from_millis(retry_delay_s));
            }
        }
        if remaining_polls < 1 {
            break;
        }
    }
    panic!("[ poll_for_write_lock ] Timed out waiting for a lock!")
}

fn poll_for_read_lock<T>(arc_rwlock: &std::sync::Arc::<std::sync::RwLock::<T>>, num_retries: usize, retry_delay_s: u64) -> std::sync::RwLockReadGuard<T> {
    let mut remaining_polls = num_retries;
    loop {
        remaining_polls -= 1;
        match std::sync::RwLock::read(&arc_rwlock) {
            Ok(rwlock) => {
                return rwlock;
            }
            Err(e) => {
                eprintln!("[ poll_for_write_lock ] {:?}", e);
                std::thread::sleep(std::time::Duration::from_millis(retry_delay_s));
            }
        }
        if remaining_polls < 1 {
            break;
        }
    }
    panic!("[ poll_for_write_lock ] Timed out waiting for a lock!")
}


fn determine_if_we_have_local_gpu(mut commands: Commands) {

    commands.spawn_task(|| async move {

        let sentinel_val = std::collections::HashMap::<String, u32>::new();
        if let Err(e) = oliana_lib::files::set_cache_file_server_proc_restart_data(&sentinel_val) {
            eprintln!("{}:{} {:?}", file!(), line!(), e);
        }

        tokio::time::sleep(std::time::Duration::from_millis(1300)).await; // Allow one 1/2 tick for file to be cleared

        let t0_restarts = tally_server_subproc_restarts();

        tokio::time::sleep(std::time::Duration::from_millis(3 * 2600)).await;

        let t1_restarts = tally_server_subproc_restarts();

        eprintln!("t0_restarts={t0_restarts} t1_restarts={t1_restarts}");

        if t1_restarts > t0_restarts {
            if let Ok(mut globals_wl) = GLOBALS.try_write() {
                let available_server = scan_for_an_open_tcp_port_at(&[
                    "127.0.0.1:8011",
                    // TODO expand list, read an env var, etc.
                ]);
                if available_server.len() > 0 {
                    globals_wl.server_url = available_server;
                    eprintln!("Connecting to server {} because our local server sub-processes are not starting up!", &globals_wl.server_url);
                }
            }
            // After releasing the write lock we can safely tell cleanup_child_procs() to clean-up the server
            cleanup_child_procs();
            eprintln!("Done stopping local tools, now using remote ones if a server was available.");
        }


        Ok(())
    });

/*    .add(|w: &mut World| {
        w.send_event(ReadyToProcessOnServerEvent("".into()));
    });
*/
}

fn scan_for_an_open_tcp_port_at(servers: &[&str]) -> String {
    let mut picked = String::new();
    for server in servers {
        match std::net::TcpStream::connect(server) {
            Ok(_conn) => {
                picked = server.to_string();
            }
            Err(e) => {
                eprintln!("{}:{} {:?}", file!(), line!(), e);
            }
        }
    }
    return picked;
}

fn tally_server_subproc_restarts() -> usize {
    let mut num_restarts: usize = 0;
    match oliana_lib::files::get_cache_file_server_proc_restart_data() {
        Ok(data) => {
            for (_k,v) in data.into_iter() {
                num_restarts += v as usize;
            }
        }
        Err(e) => {
            eprintln!("{}:{} {:?}", file!(), line!(), e);
        }
    }
    return num_restarts;
}


/*#[derive(Debug, Clone, Default, bevy::ecs::system::Resource)]
pub struct OllamaResource {
    pub ollama_inst: std::sync::Arc<std::sync::RwLock<ollama_rs::Ollama>>,
}*/


// first string is type of prompt, second string is prompt text; TODO possible args to add configs that go to oliana_text and oliana_images
#[derive(Debug, bevy::ecs::event::Event)]
pub struct PromptToAI(String, String);

// first string is type of prompt, second string is prompt reply. if "text" second string is simply the string, if "image" the second string is a file path to a .png.
#[derive(Debug, bevy::ecs::event::Event)]
pub struct ResponseFromAI(String, String);


// A unit struct to help identify the Ollama Reply UI component, since there may be many Text components
#[derive(Component)]
struct LLM_ReplyText;

// A unit struct to help identify the server URL text in the upper-right of the UI
#[derive(Component)]
struct Server_URL;







fn reset_scroll(
    q: Query<&Interaction, Changed<Interaction>>,
    mut scrolls_q: Query<&mut ScrollableContent>,
) {
    let Ok(mut scroll) = scrolls_q.get_single_mut() else {
        eprintln!("scrolls_q = returned None!");
        return;
    };
    for interaction in q.iter() {
        // eprintln!("interaction = {:?}", interaction);
        if interaction != &Interaction::Pressed {
            continue;
        }
        /*match action {
            ScrollButton::MoveToTop => scroll.scroll_to_top(),
            ScrollButton::MoveToBottom => scroll.scroll_to_bottom(),
        }*/
    }
}

