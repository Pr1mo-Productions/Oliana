
// See docs for clap's derive implementations at
//   https://docs.rs/clap/latest/clap/_derive/index.html#overview
#[derive(Debug, Clone, clap::Parser, Default, bevy::ecs::system::Resource)]
pub struct Args {
    /// Amount of verbosity in printed status messages; can be specified multiple times (ie "-v", "-vv", "-vvv" for greater verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// If set, every random-number generator will use this as their seed to allow completely deterministic AI runs.
    #[arg(short, long)]
    pub random_seed: Option<usize>,

}

impl Args {
    pub fn update_from_env(&mut self) {
        if self.random_seed.is_none() {
            if let Ok(var_txt) = std::env::var("RANDOM_SEED") {
                if var_txt.len() > 0 {
                    if let Ok(val) = var_txt.parse() {
                        eprintln!("Using random_seed = {:?}", var_txt);
                        self.random_seed = Some(val);
                    }
                }
            }
        }
    }
}


pub struct Globals {
    pub server_proc: Option<std::process::Child>,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            server_proc: None,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {

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

        let oliana_server_bin = oliana_lib::files::find_newest_mtime_bin_under_folder(&expected_bin_directory, "oliana_server")?;

        eprintln!("OLIANA_BIN_DIR={:?}", &expected_bin_directory);
        eprintln!("OLIANA_TRACKED_PROC_DIR={:?}", &track_proc_dir);
        eprintln!("Spawning {:?}", &oliana_server_bin);

        let child = std::process::Command::new(&oliana_server_bin)
                        //.args(&[])
                        .env("OLIANA_TRACKED_PROC_DIR", track_proc_dir)
                        .env("OLIANA_BIN_DIR", expected_bin_directory)
                        .spawn()?;

        self.server_proc = Some(child);

        Ok(())
    }
}


