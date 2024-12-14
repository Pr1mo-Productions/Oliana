
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

