
pub fn duration_to_display_str(d: &std::time::Duration) -> String {
  let total_millis = d.as_millis();
  let ms = total_millis % 1000;
  let s = (total_millis / 1000) % 60;
  let m = (total_millis / (1000 * 60)) % 60;
  let h = total_millis / (1000 * 60 * 60) /* % 24 */;
  if h > 0 {
    format!("{:0>2}h {:0>2}m {:0>2}s {:0>3}ms", h, m, s, ms)
  }
  else if m > 0 {
    format!("{:0>2}m {:0>2}s {:0>3}ms", m, s, ms)
  }
  else if s > 0 {
    format!("{:0>2}s {:0>3}ms", s, ms)
  }
  else {
    format!("{:0>3}ms", ms)
  }
}


