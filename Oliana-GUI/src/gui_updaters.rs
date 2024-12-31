
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
        info!("{:?} submitted: {}", event.entity, event.value);
        event_writer.send(gui_structs::PromptToAI("text".into(), event.value.clone()));
    }
}


pub fn read_ai_response_events(
    mut event_reader: EventReader<gui_structs::ResponseFromAI>,
    mut query: Query<&mut Text, With<gui_structs::LLM_ReplyText>>
) {
    for ev in event_reader.read() {
        eprintln!("{}:{} Event {:?} recieved!", file!(), line!(), ev);
        let event_type = ev.0.to_string();
        match event_type.as_str() {
            "text" => {
                if ev.1 == CLEAR_TOKEN {
                    // Clear the screen
                    for mut text in &mut query { // We'll only ever have 1 section of text rendered
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

                    for mut text in &mut query { // Append to existing content in support of a streaming design.
                        **text = format!("{}{}", text.as_str(), renderable_string);
                    }
                }
            }
            unk => {
                eprintln!("{}:{} UNKNOWN EVENT TYPE {:?}", file!(), line!(), &event_type);
            }
        }
    }
}

pub fn read_ai_prompt_events(
    mut commands: Commands,
    mut event_reader: EventReader<gui_structs::PromptToAI>,
    // mut event_writer: EventWriter<ResponseFromAI>,
) {

    for ev in event_reader.read() {
        let event_type = ev.0.to_string();
        match event_type.as_str() {
            "text" => {
                let ev_txt = ev.1.to_string();
                eprintln!("Passing this text prompt to AI: {:?}", &ev_txt);

                commands.spawn_task(|| async move {

                    let r = bevy_defer::access::AsyncWorld.send_event(gui_structs::ResponseFromAI("text".into(), CLEAR_TOKEN.to_string() ));
                    if let Err(e) = r {
                        eprintln!("{}:{} {:?}", file!(), line!(), e);
                    }

                    eprintln!("{}:{} AT", file!(), line!()); bevy_defer::access::AsyncWorld.sleep(0.05).await; // We don't know the deadlock cause, so we throw in await points between state changes as a guess.

                    let mut server_url = String::new();
                    if let Ok(mut globals_rl) = GLOBALS.try_read() {
                        server_url.push_str(&globals_rl.server_url);
                    }

                    eprintln!("{}:{} AT", file!(), line!()); bevy_defer::access::AsyncWorld.sleep(0.05).await; // We don't know the deadlock cause, so we throw in await points between state changes as a guess.

                    let mut transport = tarpc::serde_transport::tcp::connect(server_url, tarpc::tokio_serde::formats::Bincode::default);
                    eprintln!("{}:{} AT", file!(), line!());
                    transport.config_mut().max_frame_length(usize::MAX);
                    eprintln!("{}:{} AT", file!(), line!());
                    match transport.await { // This line is where we deadlock! TODO fixme
                        Ok(transport) => {
                            eprintln!("{}:{} AT", file!(), line!());
                            let client = oliana_server_lib::OlianaClient::new(tarpc::client::Config::default(), transport).spawn();
                            eprintln!("{}:{} AT", file!(), line!());
                            let mut generate_text_has_begun = false;
                            match client.generate_text_begin(tarpc::context::current(),
                                "You are an ancient storytelling diety named Olly who answers in parables and short stories.".into(),
                                ev_txt.into()
                            ).await {
                                Ok(response) => {
                                    eprintln!("[ generate_text_begin ] response = {}", &response);
                                    generate_text_has_begun = true;
                                },
                                Err(e) => {
                                    let msg = format!("{}:{} {:?}", file!(), line!(), e);
                                    eprintln!("{}", &msg);
                                    eprintln!("{}:{} AT", file!(), line!()); bevy_defer::access::AsyncWorld.sleep(0.05).await; // We don't know the deadlock cause, so we throw in await points between state changes as a guess.
                                    let r = bevy_defer::access::AsyncWorld.send_event(gui_structs::ResponseFromAI("text".into(), CLEAR_TOKEN.to_string() ));
                                    if let Err(e) = r {
                                        eprintln!("{}:{} {:?}", file!(), line!(), e);
                                    }
                                    eprintln!("{}:{} AT", file!(), line!()); bevy_defer::access::AsyncWorld.sleep(0.05).await; // We don't know the deadlock cause, so we throw in await points between state changes as a guess.
                                }
                            }

                            if generate_text_has_begun {
                                // Poll continuously, sending state up to the GUI text.
                                // TODO the LAST event in this does not return None as expected, so we do not exit smoothly and we block the UI thread!
                                let mut remaining_allowed_errs: isize = 3;
                                loop {
                                    if remaining_allowed_errs < 1 {
                                        break;
                                    }
                                    eprintln!("BEFORE tokio::time::timeout(std::time::Duration::from_millis(900), client.generate_text_next_token(tarpc::context::current())).await");
                                    match async_std::future::timeout(std::time::Duration::from_millis(1200), client.generate_text_next_token(tarpc::context::current())).await {
                                        Ok(Ok(Some(next_token))) => {
                                          eprint!("{}", &next_token);
                                          eprintln!("{}:{} AT", file!(), line!()); bevy_defer::access::AsyncWorld.sleep(0.01).await; // We don't know the deadlock cause, so we throw in await points between state changes as a guess.
                                          let r = bevy_defer::access::AsyncWorld.send_event(gui_structs::ResponseFromAI("text".into(), next_token.to_string() ));
                                          if let Err(e) = r {
                                            eprintln!("{}:{} {:?}", file!(), line!(), e);
                                          }
                                          eprintln!("{}:{} AT", file!(), line!()); bevy_defer::access::AsyncWorld.sleep(0.01).await; // We don't know the deadlock cause, so we throw in await points between state changes as a guess.
                                        }
                                        Ok(Ok(None)) => {
                                          remaining_allowed_errs -= 10;
                                          eprintln!("{}:{} AT", file!(), line!());
                                        }
                                        Ok(Err(server_err)) => {
                                          remaining_allowed_errs -= 1;
                                          eprintln!("{}:{} {:?}", file!(), line!(), server_err);
                                        }
                                        Err(timeout_err) => {
                                          remaining_allowed_errs -= 1;
                                          eprintln!("{}:{} {:?}", file!(), line!(), timeout_err);
                                        }
                                    }
                                    eprintln!("AFTER tokio::time::timeout(std::time::Duration::from_millis(900), client.generate_text_next_token(tarpc::context::current())).await remaining_allowed_errs={remaining_allowed_errs}");
                                }
                                eprintln!("Done with client.generate_text_next_token!");
                            }

                        }
                        Err(e) => {
                            let msg = format!("{}:{} {:?}", file!(), line!(), e);
                            eprintln!("{}", &msg);
                            eprintln!("{}:{} AT", file!(), line!()); bevy_defer::access::AsyncWorld.sleep(0.05).await; // We don't know the deadlock cause, so we throw in await points between state changes as a guess.
                            let r = bevy_defer::access::AsyncWorld.send_event(gui_structs::ResponseFromAI("text".into(), CLEAR_TOKEN.to_string() ));
                            if let Err(e) = r {
                                eprintln!("{}:{} {:?}", file!(), line!(), e);
                            }
                            eprintln!("{}:{} AT", file!(), line!()); bevy_defer::access::AsyncWorld.sleep(0.05).await; // We don't know the deadlock cause, so we throw in await points between state changes as a guess.
                        }
                    }

                    Ok(())
                });
            }
            unk => {
                eprintln!("{}:{} UNKNOWN EVENT TYPE {:?}", file!(), line!(), &event_type);
            }
        }
    }
}
