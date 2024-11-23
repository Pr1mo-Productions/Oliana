
// See docs for clap's derive implementations at
//   https://docs.rs/clap/latest/clap/_derive/index.html#overview
#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Amount of verbosity in printed status messages; can be specified multiple times (ie "-v", "-vv", "-vvv" for greater verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

}
