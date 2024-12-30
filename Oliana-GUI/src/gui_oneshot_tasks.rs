use crate::*;


pub fn determine_if_we_have_local_gpu(mut commands: Commands) {

    commands.spawn_task(|| async move {

        let sentinel_val = std::collections::HashMap::<String, u32>::new();
        if let Err(e) = oliana_lib::files::set_cache_file_server_proc_restart_data(&sentinel_val) {
            eprintln!("{}:{} {:?}", file!(), line!(), e);
        }

        tokio::time::sleep(std::time::Duration::from_millis(1400)).await; // Allow one 1/2 tick for file to be cleared

        let t0_restarts = tally_server_subproc_restarts();

        tokio::time::sleep(std::time::Duration::from_millis(3 * 2600)).await;

        let t1_restarts = tally_server_subproc_restarts();

        let expected_num_subprocs = tally_expected_num_subprocs();

        eprintln!("t0_restarts={t0_restarts} t1_restarts={t1_restarts}");

        if t1_restarts > expected_num_subprocs && t1_restarts > t0_restarts {
            eprintln!("We think we do not have GPU hardware because t1_restarts={t1_restarts} > t0_restarts={t0_restarts} (expected expected_num_subprocs={expected_num_subprocs}");
            let mut maybe_tokio_rt: Option<tokio::runtime::Handle> = None;
            if let Ok(mut globals_wl) = GLOBALS.try_write() {
                maybe_tokio_rt = globals_wl.tokio_rt.clone();
            }
            // After releasing the write lock we can safely tell cleanup_child_procs() to clean-up the server
            if let Some(tokio_rt) = maybe_tokio_rt {
                // Because network + process killing is slow we send it to a background tokio thread
                tokio_rt.spawn(async move {
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
                    // GLOBALS.try_write has been released, so cleanup_child_procs can grab it
                    cleanup_child_procs();
                    eprintln!("Done stopping local tools, now using remote ones if a server was available.");

                });
            }
        }
        else {
          eprintln!("We know our local AI tools are working, no longer checking process restart counts!");
        }


        Ok(())
    });
}

pub fn scan_for_an_open_tcp_port_at(servers: &[&str]) -> String {
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

pub fn tally_server_subproc_restarts() -> usize {
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

pub fn tally_expected_num_subprocs() -> usize {
    let mut num_subprocs: usize = 0;
    match oliana_lib::files::get_cache_file_server_proc_restart_data() {
        Ok(data) => {
            for (_k,_v) in data.into_iter() {
                num_subprocs += 1;
            }
        }
        Err(e) => {
            eprintln!("{}:{} {:?}", file!(), line!(), e);
        }
    }
    return num_subprocs;
}
