


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




#[derive(Debug)]
pub struct LocatedError {
    pub inner: Box<dyn std::error::Error>,
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
    pub addtl_msg: String,
}

impl std::error::Error for LocatedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

impl std::fmt::Display for LocatedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} from {}:{} ({})", self.inner, self.file, self.line, &self.addtl_msg)
    }
}

// The core idea: convenience macro to create the structure
#[macro_export]
macro_rules! eloc {
    () => {
        |e| crate::utils::LocatedError { inner: e.into(), file: file!(), line: line!(), column: column!(), addtl_msg: String::new() }
    };
    ($msg:expr) => {
        |e| crate::utils::LocatedError { inner: e.into(), file: file!(), line: line!(), column: column!(), addtl_msg: $msg }
    };
}

pub(crate) use eloc;


