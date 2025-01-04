
use crate::*;

pub fn make_visible(mut window: Query<&mut Window>, frames: Res<FrameCount>) {
    // The delay may be different for your app or system.
    if frames.0 == 3 {
        // At this point the gpu is ready to show the app so we can make the window visible.
        // Alternatively, you could toggle the visibility in Startup.
        // It will work, but it will have one white frame before it starts rendering
        window.single_mut().visible = true;
    }
}

pub fn render_server_url_in_use(mut window: Query<&mut Window>, frames: Res<FrameCount>, mut query: Query<&mut Text, With<gui_structs::Server_URL>>) {
    if frames.0 % 24 == 0 {
        let server_pcie_devices = ask_server_for_pci_devices(); // ask_server_for_pci_devices needs try_write() and doing that within a try_read() always fails
        if let Ok(globals_rl) = GLOBALS.try_read() {
            let server_url: String = globals_rl.server_url.clone();
            let mut server_txt = format!("Server: {}\n", &server_url);
            let num_devices = server_pcie_devices.len();
            for (i, device_name) in server_pcie_devices.iter().enumerate() {
                server_txt.push_str(&format!("({}) {}", i, &device_name));
                if i != num_devices-1 {
                    server_txt.push_str(" // ");
                }
            }
            for mut text in &mut query { // Append to existing content in support of a streaming design.
                **text = server_txt.clone();
            }
        }
    }
}

pub fn ask_server_for_pci_devices() -> Vec<String> {
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
        // We WILL NOT return the reply immediately; instead we tell the tokio thread pool to make the service request & we write to GLOBALS.server_pcie_devices and allow future ticks to read into the GUI
        let mut maybe_tokio_rt: Option<tokio::runtime::Handle> = None;
        if let Ok(mut globals_wl) = GLOBALS.try_write() {
            maybe_tokio_rt = globals_wl.tokio_rt.clone();
        }
        else {
            eprintln!("{}:{} GLOBALS.try_write() cannot be aquired!", file!(), line!() );
        }
        if let Some(tokio_rt) = maybe_tokio_rt {
            // This work happens off the GUI thread and eventually globals_wl.server_pcie_devices will be filled
            tokio_rt.spawn(async move {
                let mut pcie_devices = vec![];
                if let Err(e) = ask_server_for_pci_devices_async(&server_url, &mut pcie_devices).await {
                    eprintln!("{}:{} {:?}", file!(), line!(), e);
                }
                if pcie_devices.len() > 0 {
                    if let Ok(mut globals_wl) = GLOBALS.try_write() {
                        let server_url_clone = globals_wl.server_url.clone();
                        globals_wl.server_pcie_devices.insert(server_url_clone, pcie_devices.clone() );
                    }
                }
            });
        }

    }
    return pcie_devices;
}

pub async fn ask_server_for_pci_devices_async(server_url: &str, pcie_devices: &mut Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut transport = tarpc::serde_transport::tcp::connect(server_url, tarpc::tokio_serde::formats::Bincode::default);
    transport.config_mut().max_frame_length(usize::MAX);

    let client = oliana_server_lib::OlianaClient::new(tarpc::client::Config::default(), transport.await?).spawn();

    let mut hardware_names = client.fetch_pci_hw_device_names(tarpc::context::current()).await?;

    pcie_devices.append(&mut hardware_names);

    Ok(())
}


pub fn text_listener(mut events: EventReader<TextInputSubmitEvent>, mut event_writer: EventWriter<gui_structs::PromptToAI>,) {
    for event in events.read() {
        if cfg!(debug_assertions) {
            info!("{:?} submitted: {}", event.entity, event.value);
        }
        event_writer.send(gui_structs::PromptToAI("text".into(), event.value.clone()));
    }
}


pub fn read_ai_response_events(
    mut event_reader: EventReader<gui_structs::ResponseFromAI>,
    mut llm_reply_text_q: Query<&mut Text, With<gui_structs::LLM_ReplyText>>,
    mut scrollable_text_q: Query<&mut ScrollableContent>,
    mut bg_sprite_q: Query<&mut Sprite, With<gui_structs::Background_Image>>,
    asset_server: Res<AssetServer>,
) {
    for ev in event_reader.read() {
        if cfg!(debug_assertions) {
            eprintln!("{}:{} Event {:?} recieved!", file!(), line!(), ev);
        }
        let event_type = ev.0.to_string();
        match event_type.as_str() {
            "text" => {
                if ev.1 == CLEAR_TOKEN {
                    // Clear the screen
                    for mut text in &mut llm_reply_text_q { // We'll only ever have 1 section of text rendered
                        **text = String::new();
                    }
                }
                else {
                    let renderable_string = ev.1.to_string();
                    let has_begin_line_ending = renderable_string.starts_with("\n") || renderable_string.starts_with("\r\n");
                    let has_end_line_ending = renderable_string.ends_with("\n") || renderable_string.ends_with("\r\n");

                    let mut renderable_string = deunicode::deunicode(&renderable_string); // Language models can produce hard-to-render glyphs which we manually remove here.

                    if has_begin_line_ending && !(renderable_string.starts_with("\n") || renderable_string.starts_with("\r\n")) {
                        renderable_string = format!("\n{renderable_string}");
                    }
                    if has_end_line_ending && !(renderable_string.ends_with("\n") || renderable_string.ends_with("\r\n")) {
                        renderable_string = format!("{renderable_string}\n");
                    }

                    let renderable_string = renderable_string;

                    for mut text in &mut llm_reply_text_q { // Append to existing content in support of a streaming design.
                        **text = format!("{}{}", text.as_str(), renderable_string);
                    }
                    for mut scrollable_area in &mut scrollable_text_q {
                        scrollable_area.scroll_to_bottom();
                    }
                }
            }
            "image" => {
                let png_file_path = ev.1.to_string();
                let img_handle: Handle<Image> = asset_server.load(png_file_path);
                for mut bg_sprite in &mut bg_sprite_q {
                    bg_sprite.image = img_handle.clone();
                }
            }
            unk => {
                eprintln!("{}:{} UNKNOWN EVENT TYPE {:?} ev={:?}", file!(), line!(), &event_type, &ev);
            }
        }
    }
}

pub fn read_ai_prompt_events(
    mut commands: Commands,
    mut event_reader: EventReader<gui_structs::PromptToAI>,
) {

    for ev in event_reader.read() {
        let event_type = ev.0.to_string();
        match event_type.as_str() {
            "text" => {
                let ev_txt = ev.1.to_string();
                eprintln!("Passing this text prompt to AI: {:?}", &ev_txt);
                let rt = if let Ok(globals_rl) = GLOBALS.read() {
                    globals_rl.clone_tokio_rt()
                } else { panic!("Cannot read globals at this time!")};

                rt.spawn(async move {

                    if let Ok(mut globals_wl) = GLOBALS.write() {
                        globals_wl.response_from_ai_events.push(
                            gui_structs::ResponseFromAI("text".into(), CLEAR_TOKEN.to_string() )
                        );
                    }

                    let mut server_url = String::new();
                    if let Ok(mut globals_rl) = GLOBALS.read() {
                        server_url.push_str(&globals_rl.server_url);
                    }

                    let mut transport = tarpc::serde_transport::tcp::connect(server_url, tarpc::tokio_serde::formats::Bincode::default);
                    transport.config_mut().max_frame_length(usize::MAX);
                    match transport.await { // This line is where we deadlock! TODO fixme
                        Ok(transport) => {

                            let client = oliana_server_lib::OlianaClient::new(tarpc::client::Config::default(), transport).spawn();

                            let mut generate_text_has_begun = false;
                            let mut generate_image_must_be_loaded = false;

                            match client.generate_text_begin(tarpc::context::current(),
                                "You are an ancient storytelling diety named Olly who answers in parables and short stories.".into(),
                                ev_txt.clone().into()
                            ).await {
                                Ok(response) => {
                                    eprintln!("[ generate_text_begin ] response = {}", &response);
                                    generate_text_has_begun = true;
                                },
                                Err(e) => {
                                    let msg = format!("{}:{} {:?}", file!(), line!(), e);
                                    eprintln!("{}", &msg);

                                    if let Ok(mut globals_wl) = GLOBALS.write() {
                                        globals_wl.response_from_ai_events.push(
                                            gui_structs::ResponseFromAI("text".into(), CLEAR_TOKEN.to_string() )
                                        );
                                    }

                                }
                            }

                            match client.generate_image_begin(tarpc::context::current(),
                                ev_txt.clone().into(),
                                "".to_string(), 3.5, 12
                            ).await {
                                Ok(response) => {
                                    eprintln!("[ generate_image_begin ] response = {}", &response);
                                    generate_image_must_be_loaded = true;
                                },
                                Err(e) => {
                                    let msg = format!("{}:{} {:?}", file!(), line!(), e);
                                    eprintln!("{}", &msg);
                                }
                            }

                            if generate_text_has_begun {
                                // Poll continuously, sending state up to the GUI text.
                                // TODO the LAST event in this does not return None as expected, so we do not exit smoothly and we block the UI thread!
                                let mut remaining_allowed_errs: isize = 12;
                                loop {
                                    if remaining_allowed_errs < 1 {
                                        break;
                                    }
                                    match client.generate_text_next_token(tarpc::context::current()).await {
                                        Ok(Some(next_token)) => {
                                          if let Ok(mut globals_wl) = GLOBALS.write() {
                                            globals_wl.response_from_ai_events.push(
                                              gui_structs::ResponseFromAI("text".into(), next_token.to_string() )
                                            );
                                          }
                                        }
                                        Ok(None) => {
                                          remaining_allowed_errs -= 10;
                                        }
                                        Err(server_err) => {
                                          remaining_allowed_errs -= 1;
                                        }
                                    }

                                    if generate_image_must_be_loaded {
                                        match client.generate_image_result_exists(tarpc::context::current()).await {
                                            Ok(result_exists_bool) => {
                                                if result_exists_bool {
                                                    read_image_from_server_and_push_event_to_globals(&client).await;
                                                    generate_image_must_be_loaded = false;
                                                }
                                            }
                                            Err(server_err) => {
                                                remaining_allowed_errs -= 1;
                                            }
                                        }
                                    }

                                }
                            }

                            if generate_image_must_be_loaded {
                                read_image_from_server_and_push_event_to_globals(&client).await;
                            }

                        }
                        Err(e) => {
                            let msg = format!("{}:{} {:?}", file!(), line!(), e);
                            eprintln!("{}", &msg);
                            if let Ok(mut globals_wl) = GLOBALS.write() {
                                globals_wl.response_from_ai_events.push(
                                    gui_structs::ResponseFromAI("text".into(), CLEAR_TOKEN.to_string() )
                                );
                                globals_wl.response_from_ai_events.push(
                                    gui_structs::ResponseFromAI("text".into(), msg.clone() )
                                );
                            }
                        }
                    }
                });
            }
            unk => {
                eprintln!("{}:{} UNKNOWN EVENT TYPE {:?}", file!(), line!(), &event_type);
            }
        }
    }
}

async fn read_image_from_server_and_push_event_to_globals(client: &oliana_server_lib::OlianaClient) {
    match client.generate_image_get_result(tarpc::context::current()).await {
        Ok(png_vec_u8) => {
            eprintln!("Read {} bytes of PNG image from AI server!", png_vec_u8.len());
            let tmp_png_file_path = oliana_lib::files::get_cache_file("tmp.png").expect("Fatal Filesystem error // todo remove me");
            if let Err(e) = tokio::fs::write(&tmp_png_file_path, &png_vec_u8[..]).await {
                eprintln!("{}:{} {:?}", file!(), line!(), e);
            }
            if let Ok(mut globals_wl) = GLOBALS.write() {
              globals_wl.response_from_ai_events.push(
                gui_structs::ResponseFromAI("image".into(), tmp_png_file_path.to_string_lossy().to_string())
              );
            }
        }
        Err(e) => {
            let msg = format!("{}:{} {:?}", file!(), line!(), e);
            eprintln!("{}", &msg);
        }
    }
}


// Runs every 50ms, moves GLOBALS events to Bevy ECS
pub fn drain_global_events_to_bevy(mut event_writer: EventWriter<gui_structs::ResponseFromAI>) {
    if let Ok(mut globals_wl) = GLOBALS.try_write() {
        for global_item in globals_wl.response_from_ai_events.drain(0..) {
            event_writer.send(global_item);
        }
    }
}

