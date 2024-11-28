
use bevy::{
    core::FrameCount,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{CursorGrabMode, PresentMode, WindowLevel, WindowTheme},
};

use bevy_simple_text_input::{
    TextInputBundle, TextInputInactive, TextInputPlugin, TextInputSystem, TextInputSubmitEvent
};

const BORDER_COLOR_ACTIVE: Color = Color::srgb(0.75, 0.52, 0.99);
const BORDER_COLOR_INACTIVE: Color = Color::srgb(0.25, 0.25, 0.25);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const BACKGROUND_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);



pub async fn open_gui_window(cli_args: &crate::cli::Args) -> Result<(), Box<dyn std::error::Error>> {
  App::new()
    .add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Oliana".into(),
                name: Some("Oliana".into()),
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
        }),
        LogDiagnosticsPlugin::default(),
        FrameTimeDiagnosticsPlugin,
    ))
    .add_plugins(bevy_defer::AsyncPlugin::default_settings())
    .add_systems(
        Update,
        (
            change_title,
            toggle_theme,
            //toggle_cursor,
            toggle_vsync,
            toggle_window_controls,
            switch_level,
            make_visible,
        ),
    )
    .add_plugins(TextInputPlugin)
    .add_systems(Startup, setup)
    .insert_resource((*cli_args).clone())
    .add_systems(Startup, create_ai_engine)
    .add_systems(Update, focus.before(TextInputSystem))
    .add_systems(Update, text_listener.after(TextInputSystem))
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

/// This system toggles the vsync mode when pressing the button V.
/// You'll see fps increase displayed in the console.
fn toggle_vsync(input: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    if input.just_pressed(KeyCode::KeyV) {
        let mut window = windows.single_mut();

        window.present_mode = if matches!(window.present_mode, PresentMode::AutoVsync) {
            PresentMode::AutoNoVsync
        } else {
            PresentMode::AutoVsync
        };
        info!("PRESENT_MODE: {:?}", window.present_mode);
    }
}

/// This system switches the window level when pressing the T button
/// You'll notice it won't be covered by other windows, or will be covered by all the other
/// windows depending on the level.
///
/// This feature only works on some platforms. Please check the
/// [documentation](https://docs.rs/bevy/latest/bevy/prelude/struct.Window.html#structfield.window_level)
/// for more details.

fn switch_level(input: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    if input.just_pressed(KeyCode::KeyT) {
        let mut window = windows.single_mut();

        window.window_level = match window.window_level {
            WindowLevel::AlwaysOnBottom => WindowLevel::Normal,
            WindowLevel::Normal => WindowLevel::AlwaysOnTop,
            WindowLevel::AlwaysOnTop => WindowLevel::AlwaysOnBottom,
        };
        info!("WINDOW_LEVEL: {:?}", window.window_level);
    }
}

/// This system toggles the window controls when pressing buttons 1, 2 and 3
///
/// This feature only works on some platforms. Please check the
/// [documentation](https://docs.rs/bevy/latest/bevy/prelude/struct.Window.html#structfield.enabled_buttons)
/// for more details.
fn toggle_window_controls(input: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    let toggle_minimize = input.just_pressed(KeyCode::Digit1);
    let toggle_maximize = input.just_pressed(KeyCode::Digit2);
    let toggle_close = input.just_pressed(KeyCode::Digit3);

    if toggle_minimize || toggle_maximize || toggle_close {
        let mut window = windows.single_mut();

        if toggle_minimize {
            window.enabled_buttons.minimize = !window.enabled_buttons.minimize;
        }
        if toggle_maximize {
            window.enabled_buttons.maximize = !window.enabled_buttons.maximize;
        }
        if toggle_close {
            window.enabled_buttons.close = !window.enabled_buttons.close;
        }
    }
}

/// This system will then change the title during execution
fn change_title(mut windows: Query<&mut Window>, time: Res<Time>) {
    let mut window = windows.single_mut();
    window.title = format!(
        "Seconds since startup: {}",
        time.elapsed().as_secs_f32().round()
    );
}

/* // Useful - this is how we lock the cursor if we ever want WASD-style movement mechanics w/ cursor for camera control
fn toggle_cursor(mut windows: Query<&mut Window>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::Space) {
        let mut window = windows.single_mut();

        window.cursor.visible = !window.cursor.visible;
        window.cursor.grab_mode = match window.cursor.grab_mode {
            CursorGrabMode::None => CursorGrabMode::Locked,
            CursorGrabMode::Locked | CursorGrabMode::Confined => CursorGrabMode::None,
        };
    }
}*/


// This system will toggle the color theme used by the window
fn toggle_theme(mut windows: Query<&mut Window>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::KeyF) {
        let mut window = windows.single_mut();

        if let Some(current_theme) = window.window_theme {
            window.window_theme = match current_theme {
                WindowTheme::Light => Some(WindowTheme::Dark),
                WindowTheme::Dark => Some(WindowTheme::Light),
            };
        }
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

    // Text with one section
    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            "hello\nbevy!",
            TextStyle {
                // This font is loaded and will be used instead of the default font.
                // font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 32.0,
                ..default()
            },
        ) // Set the justification of the Text
        .with_text_justify(JustifyText::Center)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            left: Val::Px(4.0),
            ..default()
        }),
        //ColorText,
    ));




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

fn text_listener(mut events: EventReader<TextInputSubmitEvent>, ) {
    for event in events.read() {
        info!("{:?} submitted: {}", event.entity, event.value);
    }
}

fn create_ai_engine(cli_args: Res<crate::cli::Args>, mut commands: Commands) {
    use bevy_defer::AsyncCommandsExtension;
    //let handle = tokio::runtime::Handle::current(); // Safety: Bevy runs in one of tokio's async methods, therefore MUST have an EnterGuard setup.
    //let ai_engine = handle.block_on(AI_Engine::new(&cli_args));
    //commands.spawn(ai_engine);
    let cli_args_clone = cli_args.clone();
    commands.spawn_task(|| async move {
        let ai_engine = AI_Engine::new(&cli_args_clone).await;
        //commands.spawn(ai_engine); // TODO hmmmmm
        Ok(())
    });
}

const OLLAMA_MODEL_NAME: &'static str = "qwen2.5:7b";

#[derive(Component)]
struct AI_Engine {
    pub ollama_inst: ollama_rs::Ollama,
    pub have_verified_qwen2_5_7b_up: bool,
    pub streaming_reply_tokens: std::sync::Arc<std::sync::RwLock<Vec<String>>>,
}

impl AI_Engine {
    pub async fn new(cli_args: &crate::cli::Args) -> Self {
        let mut s = Self {
            ollama_inst: crate::ai::init_ollama_with_model_pulled(cli_args, "qwen2.5:7b").await.expect("Could not run ollama, ensure ollama[.exe] is installed an on PATH!"),
            have_verified_qwen2_5_7b_up: false,
            streaming_reply_tokens: std::sync::Arc::new(vec![].into()),
            //self_ref: None,
        };
        //s.self_ref = Some(std::sync::Arc::new(s));
        s
    }

    pub fn begin_prompting(&mut self, prompt_txt: &str) {
        let handle = tokio::runtime::Handle::current(); // Safety: Bevy runs in one of tokio's async methods, therefore MUST have an EnterGuard setup.

        handle.block_on(self.ensure_qwen2_5_7b_up());

        let ollama_dupe_client = self.ollama_inst.clone();
        let prompt_txt_one = prompt_txt.to_string();
        let streaming_reply_tokens_jh = self.streaming_reply_tokens.clone();
        let join_handle = handle.spawn({
            async move {
                match ollama_dupe_client.generate(ollama_rs::generation::completion::request::GenerationRequest::new(OLLAMA_MODEL_NAME.to_string(), prompt_txt_one)).await {
                    Ok(res) => {
                        streaming_reply_tokens_jh.write().expect("No writable ref available").push(res.response);
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                        streaming_reply_tokens_jh.write().expect("No writable ref available").push(format!("{:?}", e));
                    }
                }
            }
        });

    }

    pub async fn ensure_qwen2_5_7b_up(&mut self) {
        if self.have_verified_qwen2_5_7b_up {
            return;
        }
        if let Err(e) = self.ensure_qwen2_5_7b_up_errs().await {
            eprintln!("{:?}", e);
        }
    }
    pub async fn ensure_qwen2_5_7b_up_errs(&mut self) -> Result<(), Box<dyn std::error::Error>>  {
        match self.ollama_inst.list_local_models().await {
          Ok(local_models) => {
            /* Nop */
          }
          Err(e) => {
            eprintln!("{:#?}", crate::utils::LocatedError { inner: Box::new(e), file: file!(), line: line!(), column: column!(), addtl_msg: String::new() });

            eprintln!("Executing 'ollama serve' as a background process...");

            tokio::process::Command::new("ollama")
              .args(&["serve"])
              .kill_on_drop(false) // Prevents tokio from reaping process on Drop
              .spawn().map_err(crate::utils::eloc!())?;

            // Delay for 750ms or so
            tokio::time::sleep(std::time::Duration::from_millis(750)).await;
          }
        }

        let local_models = self.ollama_inst.list_local_models().await.map_err(crate::utils::eloc!())?;
        // eprintln!("Ollama models = {:#?}", local_models);

        /*let qwen2_5_7b_model_file = download_file_ifne(
          cli_args,
          crate::utils::get_cache_file("qwen2_5_7b.Modelfile").await?,
          "https://huggingface.co/openai-community/gpt2/raw/main/tokenizer.json"
        ).await?;*/
        // ^^ todo research so we can control our own downloads

        match self.ollama_inst.show_model_info(OLLAMA_MODEL_NAME.to_string()).await {
          Ok(model_info) => { /* unused */ },
          Err(e) => {
            eprintln!("{:#?}", crate::utils::LocatedError { inner: Box::new(e), file: file!(), line: line!(), column: column!(), addtl_msg: String::new() });
            // Spawn off a download
            eprintln!("Telling ollama to pull the model {}...", OLLAMA_MODEL_NAME);
            self.ollama_inst.pull_model(OLLAMA_MODEL_NAME.to_string(), true).await?;
            eprintln!("Done pulling {}!", OLLAMA_MODEL_NAME);
          }
        }

        self.have_verified_qwen2_5_7b_up = true;

        Ok(())
    }
}






