
pub fn get_credits_txt() -> String {
  let contributors_txt = include_str!("../../contributors.txt");
  let build_time = build_time::build_time_local!("%Y-%m-%d %H:%M:%S");
  format!("Built with Love by\n{contributors_txt}\nAt {build_time}")
}

