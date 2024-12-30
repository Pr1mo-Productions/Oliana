
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

pub fn render_server_url_in_use(mut window: Query<&mut Window>, frames: Res<FrameCount>, mut query: Query<&mut Text, With<Server_URL>>) {
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
                text.sections[0].value = server_txt.clone();
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

