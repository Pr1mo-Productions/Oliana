
use bevy::{
    core::FrameCount,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{CursorGrabMode, PresentMode, WindowLevel, WindowTheme},
};

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
      std::process::exit(1);
    }
  });

  Ok(())
}

pub async fn main_async(cli_args: &structs::Args) -> Result<(), Box<dyn std::error::Error>> {

  if let Ok(mut globals_wl) = GLOBALS.try_write() {
    if let Err(e) = globals_wl.initialize() {
        eprintln!("{}:{} {}", file!(), line!(), e);
    }
  }

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
                        maximize: false,
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

    .add_event::<OllamaIsReadyToProcessEvent>()
    .add_event::<PromptToOllamaEvent>()
    .add_event::<ResponseFromOllamaEvent>()

    .insert_resource((*cli_args).clone()) // Accept a Ref<crate::cli::Args> in your system's function to read cli args in the UI
    //.insert_resource(OllamaResource::default()) // Accept a Ref<crate::gui::OllamaResource> in your system's function to touch the Ollama stuff

    .add_systems(
        Update,
        (
            make_visible,
        ),
    )
    .add_systems(Startup, (setup,) )
    //.add_systems(Startup, (setup, setup_ollama) )
    .add_systems(Update, focus.before(TextInputSystem))
    .add_systems(Update, text_listener.after(TextInputSystem))
    .add_systems(Update, read_ollama_ready_events)
    .add_systems(Update, read_ollama_response_events)
    //.add_systems(Update, read_ollama_prompt_events)
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

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::End, // End here means "Bottom"
                    justify_content: JustifyContent::Start, // Start here means "Left"
                    padding: UiRect::all(Val::Px(4.0)),
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
                        padding: UiRect::all(Val::Px(4.0)),
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
        });

    // Text with one section; OllamaReplyText allows us to refer to the TextBundle?
    /*commands.spawn((
        // Create a TextBundle that has a Text with a single section.
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
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            left: Val::Px(4.0),
            right: Val::Px(4.0),
            bottom: Val::Px(52.0),
            ..default()
        }),
        OllamaReplyText,
    ));*/

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(4.0),
                left: Val::Px(4.0),
                right: Val::Px(4.0),
                bottom: Val::Px(56.0),
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
                /*position_type: PositionType::Absolute,
                top: Val::Px(4.0),
                left: Val::Px(4.0),
                right: Val::Px(4.0),
                bottom: Val::Px(56.0),*/
                min_width: Val::Px(200.0),
                // min_height: Val::Px(900.0),
                margin: UiRect::all(Val::Px(4.0)),
                //border: UiRect::all(Val::Px(5.0)),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            }),
            OllamaReplyText,
            ScrollableContent::default(),
        ));
    });


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

fn text_listener(mut events: EventReader<TextInputSubmitEvent>, mut event_writer: EventWriter<PromptToOllamaEvent>,) {
    for event in events.read() {
        info!("{:?} submitted: {}", event.entity, event.value);
        event_writer.send(PromptToOllamaEvent(event.value.clone()));
    }
}

fn read_ollama_ready_events(
    mut event_reader: EventReader<OllamaIsReadyToProcessEvent>,
) {
    for ev in event_reader.read() {
        eprintln!("Event {:?} recieved!", ev);
    }
}

fn read_ollama_response_events(
    mut event_reader: EventReader<ResponseFromOllamaEvent>,
    mut query: Query<&mut Text, With<OllamaReplyText>>
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
/*
fn read_ollama_prompt_events(
    mut commands: Commands,
    mut event_reader: EventReader<PromptToOllamaEvent>,
    // mut event_writer: EventWriter<ResponseFromOllamaEvent>,
    cli_args: Res<crate::cli::Args>,
    mut ollama_resource: ResMut<crate::gui::OllamaResource>,
) {
    use std::iter::Iterator;
    use futures_util::StreamExt;

    let arc_to_ollama_rwlock = ollama_resource.into_inner().ollama_inst.clone();

    let desired_model_name = cli_args.ollama_model_name.clone().unwrap_or("qwen2.5:7b".to_string());

    for ev in event_reader.read() {
        let ev_txt = ev.0.to_string();
        eprintln!("Passing this prompt to Ollama: {:?}", ev.0);

        let closure_arc_to_ollama_rwlock = arc_to_ollama_rwlock.clone();
        let closure_owned_desired_model_name = desired_model_name.to_string();

        commands.spawn_task(|| async move {
            let ollama_resource_readlock = poll_for_read_lock(&closure_arc_to_ollama_rwlock, 100, 50);

            let r = bevy_defer::access::AsyncWorld.send_event(ResponseFromOllamaEvent( CLEAR_TOKEN.to_string() ));
            if let Err(e) = r {
                eprintln!("[ read_ollama_prompt_events ] {:?}", e);
            }

            match ollama_resource_readlock.generate_stream(ollama_rs::generation::completion::request::GenerationRequest::new(closure_owned_desired_model_name, ev_txt)).await {
                Ok(mut reply_stream) => {
                    while let Some(Ok(several_responses)) = reply_stream.next().await {
                        for response in several_responses.iter() {
                            let r = bevy_defer::access::AsyncWorld.send_event(ResponseFromOllamaEvent( response.response.to_string() ));
                            if let Err(e) = r {
                                eprintln!("[ read_ollama_prompt_events ] {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("e = {:?}", e);
                    let r = bevy_defer::access::AsyncWorld.send_event(ResponseFromOllamaEvent(format!("{:?}", e)));
                    if let Err(e) = r {
                        eprintln!("[ read_ollama_prompt_events ] {:?}", e);
                    }
                }
            }

            Ok(())
        });
    }
}
*/

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

/*
fn setup_ollama(mut commands: Commands, mut ollama_resource: ResMut<crate::gui::OllamaResource>, cli_args: Res<crate::cli::Args>, mut ev_ollama_ready: EventWriter<OllamaIsReadyToProcessEvent>) {

    // The write lock is NOT dropped here, it is MOVED into the async context below.
    let owned_cli_args: crate::cli::Args = cli_args.clone();
    let arc_to_ollama_rwlock = ollama_resource.into_inner().ollama_inst.clone();
    let desired_model_name = owned_cli_args.ollama_model_name.clone().unwrap_or("qwen2.5:7b".to_string());

    eprintln!("desired_model_name = {:?}", &desired_model_name);

    commands.spawn_task(|| async move {

        let mut ollama_resource_writelock = poll_for_write_lock(&arc_to_ollama_rwlock, 100, 50);

        match crate::ai::init_ollama_with_model_pulled(&owned_cli_args, &desired_model_name).await {
            Ok(ollama_inst) => {
                *ollama_resource_writelock = ollama_inst;
            }
            Err(e) => {
                eprintln!("[ setup_ollama ] e = {:?}", e);
            }
        }

        // std::thread::sleep(std::time::Duration::from_millis(3500)); // Haha yeah we suck, but this is a good knee-jerk measurement of ^^ lotsa work upstairs

        Ok(())
    })
    .add(|w: &mut World| {
        w.send_event(OllamaIsReadyToProcessEvent());
    });
}



#[derive(Debug, Clone, Default, bevy::ecs::system::Resource)]
pub struct OllamaResource {
    pub ollama_inst: std::sync::Arc<std::sync::RwLock<ollama_rs::Ollama>>,
}
*/

#[derive(Debug, bevy::ecs::event::Event)]
pub struct OllamaIsReadyToProcessEvent();

#[derive(Debug, bevy::ecs::event::Event)]
pub struct PromptToOllamaEvent(String);

#[derive(Debug, bevy::ecs::event::Event)]
pub struct ResponseFromOllamaEvent(String);


// A unit struct to help identify the Ollama Reply UI component, since there may be many Text components
#[derive(Component)]
struct OllamaReplyText;







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

