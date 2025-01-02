
#![allow(unused_imports, unused_variables)]

use futures::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt  = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(std::cmp::max(2, num_cpus::get_physical())) // Use all host cores, unless single-cored in which case pretend to have 2
    .thread_stack_size(8 * 1024 * 1024)
    .enable_time()
    .enable_io()
    .build()?;

  rt.block_on(async {
    if let Err(e) = main_async().await {
      eprintln!("[ main_async ] {}", e);
      std::process::exit(1);
    }
  });

  Ok(())
}

async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}


// This is used for the TCP listeners to communicate liviliness to the background task which ensures sub-processes are running.
// When system is idle on Linux systems, children get SIGSTOP/CONT-ed to limit CPU use when nobody has connected for >20s.
static LAST_CLIENT_CONNECT_TIME_EPOCH_S: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);


async fn main_async() -> Result<(), Box<dyn std::error::Error>> {
    use tarpc::server::Channel;
    use oliana_server_lib::Oliana;
    use futures::StreamExt;
    use tarpc::server::incoming::Incoming;

    // TODO allow passing in configs for each of these folders; env vars are nice, config files are nicer!

    let mut expected_bin_directory = std::env::current_dir()?;
    if expected_bin_directory.join("target").exists() {
        expected_bin_directory = expected_bin_directory.join("target");
    }
    let mut track_proc_dir = expected_bin_directory.clone();

    if let Ok(env_expected_bin_dir) = std::env::var("OLIANA_BIN_DIR") {
        if std::path::Path::new(&env_expected_bin_dir).exists() {
            expected_bin_directory = env_expected_bin_dir.into();
        }
    }

    if let Ok(env_track_proc_dir) = std::env::var("OLIANA_TRACKED_PROC_DIR") {
        track_proc_dir = env_track_proc_dir.into();
    }


    let mut procs = oliana_lib::launchers::TrackedProcs::new(expected_bin_directory.clone(), track_proc_dir.clone());

    // This is where we do some general config of how & where the child processes will live.
    // Once registered, the server will regularly poll .ensure_registered_procs_running() to re-spawn anything that dies.
    let ai_workdir_images = track_proc_dir.join("image-procesing");
    let ai_workdir_text = track_proc_dir.join("text-procesing");

    if !ai_workdir_images.exists() {
        std::fs::create_dir_all(ai_workdir_images.clone()).map_err(oliana_lib::eloc!())?;
    }
    if !ai_workdir_text.exists() {
        std::fs::create_dir_all(ai_workdir_text.clone()).map_err(oliana_lib::eloc!())?;
    }

    // Delete all files in ai_workdir_images and ai_workdir_text; this prevents concurrency build-up over time
    for working_dir in [&ai_workdir_images, &ai_workdir_text] {
        let mut dir_iterator = tokio::fs::read_dir(&working_dir).await?;
        while let Some(entry) = dir_iterator.next_entry().await? {
            let entry_path = entry.path();
            if entry_path.is_file() {
                tokio::fs::remove_file(&entry_path).await?;
            }
        }
    }

    // We set & pass down this value which backends may read to avoid over-allocating eachother's slice of the GPU pie.
    let mut per_proc_mem_already_defined = false;
    if let Ok(per_proc_mem_val) = std::env::var("PER_PROC_MEM_FRACT") {
        if per_proc_mem_val.len() > 0 {
            per_proc_mem_already_defined = true;
            eprintln!("Not changing already-existing PER_PROC_MEM_FRACT value of {}", &per_proc_mem_val);
        }
    }
    if !per_proc_mem_already_defined {
        eprintln!("Setting PER_PROC_MEM_FRACT=0.40 for child processes (otherwise they will over-allocate and eat >100% of GPU memory and one will lose the race and go home cryting for more VRAM)");
        std::env::set_var(
          "PER_PROC_MEM_FRACT", "0.40"
        );
    }

    procs.register_tracked_proc("oliana_images", &[
        "--workdir", &ai_workdir_images.to_string_lossy()
    ]);

    procs.register_tracked_proc("oliana_text", &[
        "--workdir", &ai_workdir_text.to_string_lossy()
    ]);

    procs.ensure_registered_procs_running()?;

    let shareable_procs = std::sync::Arc::new(std::sync::RwLock::new(procs));
    let shareable_ipv6_ai_workdir_images = ai_workdir_images.to_string_lossy().to_string();
    let shareable_ipv6_ai_workdir_text = ai_workdir_text.to_string_lossy().to_string();
    let shareable_ipv4_ai_workdir_images = ai_workdir_images.to_string_lossy().to_string();
    let shareable_ipv4_ai_workdir_text = ai_workdir_text.to_string_lossy().to_string();

    // Start an infinite tokio task to call ensure_registered_procs_running()? every 2 seconds or so.
    let ensure_registered_procs_running_t_shareable_procs = shareable_procs.clone();
    tokio::task::spawn(async move {
        let mut ms_since_last_ensured_running: u64 = 0;
        const MS_TO_TICK_FOR: u64 = 80;
        let tick_delay_duration = std::time::Duration::from_millis(MS_TO_TICK_FOR);
        const MS_TO_RESUME_PROCS_FOR: u64 = 20;
        let resume_duration = std::time::Duration::from_millis(MS_TO_RESUME_PROCS_FOR);
        let mut last_tick_procs_should_be_stopped = false;
        loop {
            tokio::time::sleep(tick_delay_duration).await;
            ms_since_last_ensured_running += MS_TO_TICK_FOR;

            if let Ok(read_lock_guard) = ensure_registered_procs_running_t_shareable_procs.try_read() {
                if let Err(e) = read_lock_guard.resume_sigstop_procs(resume_duration) {
                    eprintln!("{}:{} {:?}", file!(), line!(), e);
                }
            }

            if ms_since_last_ensured_running > 2600 {
                if let Ok(mut write_lock_guard) = ensure_registered_procs_running_t_shareable_procs.try_write() {
                    if let Err(e) = write_lock_guard.ensure_registered_procs_running() {
                        eprintln!("Error polling ensure_registered_procs_running: {:?}", e);
                    }
                    // If it's been > 20s since last incoming message, begin SIGSTOP-ing child processes?
                    #[cfg(target_os = "linux")]
                    {
                        if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                            let last_client_connect_s = LAST_CLIENT_CONNECT_TIME_EPOCH_S.load(std::sync::atomic::Ordering::Relaxed);
                            let seconds_since_last_client_connect = duration.as_secs() - last_client_connect_s;
                            let procs_should_be_stopped = seconds_since_last_client_connect > 24;
                            if procs_should_be_stopped != last_tick_procs_should_be_stopped {
                                if procs_should_be_stopped {
                                    eprintln!("oliana_server is suspending child processes periodically because the last client connect was {} seconds ago! (procs_should_be_stopped={})", seconds_since_last_client_connect, procs_should_be_stopped);
                                }
                                else {
                                    eprintln!("oliana_server is resuming child processes because the last client connect was {} seconds ago! (procs_should_be_stopped={})", seconds_since_last_client_connect, procs_should_be_stopped);
                                }
                            }
                            write_lock_guard.set_procs_should_be_stopped(procs_should_be_stopped);
                            last_tick_procs_should_be_stopped = procs_should_be_stopped;
                        }
                    }
                }
                if let Ok(read_lock_guard) = ensure_registered_procs_running_t_shareable_procs.try_read() {
                    // Now we summarize sub-process running state and write to 2 files that other programs can poll to report subprocess status.
                    // This is primarially used so things like Oliana-GUI can tell a user "You don't have CUDA/OneAPI/<tech-of-choice>" without needing to actually EMBED <tech-of-choice> to perform the measurement.
                    let data = read_lock_guard.get_proc_restart_counts();
                    if let Err(e) = oliana_lib::files::set_cache_file_server_proc_restart_data(&data) {
                        eprintln!("{}:{} {}", file!(), line!(), e);
                    }
                    let data = read_lock_guard.get_proc_outputs();
                    if let Err(e) = oliana_lib::files::set_cache_file_server_proc_outputs_data(&data) {
                        eprintln!("{}:{} {}", file!(), line!(), e);
                    }
                }
                ms_since_last_ensured_running = 0;
            }
        }
    });

    let mut port: u16 = 9050;
    if let Ok(env_val) = std::env::var("PORT") {
        if let Ok(parsed_port_num) = env_val.parse::<u16>() {
            eprintln!("Overriding default port of {} with environment PORT={}", port, parsed_port_num);
            port = parsed_port_num;
        }
    }

    let ipv4_server_addr = (std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), port);
    let ipv6_server_addr = (std::net::IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED), port);

    println!("ipv4_server_addr = {ipv4_server_addr:?}");
    println!("ipv6_server_addr = {ipv6_server_addr:?}");
    println!("port = {port:?} (used by both ipv4 and v6 servers)");

    println!("expected_bin_directory = {expected_bin_directory:?} (Where eg oliana_images[.exe] can be found)");
    println!("track_proc_dir = {track_proc_dir:?} (Where eg oliana_images[.exe]-pid.txt may be found)");
    println!("ai_workdir_images = {ai_workdir_images:?} (Where images are generated into and read by the server)");
    println!("ai_workdir_text = {ai_workdir_text:?} (Where text is generated into and read by the server)");


    // JSON transport is provided by the json_transport tarpc module. It makes it easy
    // to start up a serde-powered json serialization strategy over TCP.
    let mut ipv6_listener = tarpc::serde_transport::tcp::listen(&ipv6_server_addr, tarpc::tokio_serde::formats::Bincode::default).await?;
    println!("Server Listening on {:?}", &ipv6_server_addr);

    // Infrastructure detail: If the Host OS has dual-stacking turned on, the above ipv6_listener will bind to both ipv6 and v4 addresses.
    //                        If the Host OS has dual-stacking turned off, we still want to explicitly launch a v4 connector to support v4 clients.
    let mut maybe_ipv4_listener = None;
    if let Ok(ipv4_listener) = tarpc::serde_transport::tcp::listen(&ipv4_server_addr, tarpc::tokio_serde::formats::Bincode::default).await {
        maybe_ipv4_listener = Some(ipv4_listener);
        println!("Server Listening on {:?}", &ipv4_server_addr);
    }

    if let Some(ref mut ipv4_listener) = maybe_ipv4_listener {
        ipv4_listener.config_mut().max_frame_length(usize::MAX);
    }
    ipv6_listener.config_mut().max_frame_length(usize::MAX);

    let mut all_futures = vec![];
    let ipv6_movable_shareable_procs = shareable_procs.clone();
    let ipv6_futures = tokio::spawn(ipv6_listener
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            .map(tarpc::server::BaseChannel::with_defaults)
            // Limit channels to 1 per IP.
            .max_channels_per_key(128, |t| t.transport().peer_addr().unwrap().ip())
            // serve is generated by the service attribute. It takes as input any type implementing
            // the generated World trait.
            .map(move |channel| {
                let server = oliana_server_lib::OlianaServer::new(
                    channel.transport().peer_addr().expect("IPv6 Client had no peer_addr!"),
                    ipv6_movable_shareable_procs.clone(),
                    &shareable_ipv6_ai_workdir_images[..],
                    &shareable_ipv6_ai_workdir_text[..]
                );
                if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    LAST_CLIENT_CONNECT_TIME_EPOCH_S.store(duration.as_secs(), std::sync::atomic::Ordering::Relaxed);
                    if let Ok(mut write_guard) = ipv6_movable_shareable_procs.write() {
                        write_guard.set_procs_should_be_stopped(false);
                    }
                }
                channel.execute(server.serve()).for_each(spawn)
            })
            // Max 10 channels.
            .buffer_unordered(10)
            .for_each(|_| async {}));

    all_futures.push(ipv6_futures);

    if let Some(ipv4_listener) = maybe_ipv4_listener {
            all_futures.push(
                tokio::spawn(ipv4_listener
                    // Ignore accept errors.
                    .filter_map(|r| future::ready(r.ok()))
                    .map(tarpc::server::BaseChannel::with_defaults)
                    // Limit channels to 1 per IP.
                    .max_channels_per_key(128, |t| t.transport().peer_addr().unwrap().ip())
                    // serve is generated by the service attribute. It takes as input any type implementing
                    // the generated World trait.
                    .map(move |channel| {
                        let server = oliana_server_lib::OlianaServer::new(
                            channel.transport().peer_addr().expect("IPv4 Client had no peer_addr!"),
                            shareable_procs.clone(),
                            &shareable_ipv4_ai_workdir_images[..],
                            &shareable_ipv4_ai_workdir_text[..]
                        );
                        if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                            LAST_CLIENT_CONNECT_TIME_EPOCH_S.store(duration.as_secs(), std::sync::atomic::Ordering::Relaxed);
                            if let Ok(mut write_guard) = shareable_procs.write() {
                                write_guard.set_procs_should_be_stopped(false);
                            }
                        }
                        channel.execute(server.serve()).for_each(spawn)
                    })
                    // Max 10 channels.
                    .buffer_unordered(10)
                    .for_each(|_| async {}))
            );
    }

    // all_futures may have 1 or two listeners
    for fut in all_futures {
        fut.await?;
    }

    Ok(())
}

