


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
        if self.addtl_msg.len() > 0 {
            write!(f, "{} from {}:{} ({})", self.inner, self.file, self.line, &self.addtl_msg)
        }
        else {
            write!(f, "{} from {}:{}", self.inner, self.file, self.line)
        }
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

#[macro_export]
macro_rules! eloc_str {
    () => {
        |e| crate::utils::LocatedError { inner: format!("{:?}", e).into(), file: file!(), line: line!(), column: column!(), addtl_msg: String::new() }
    };
    ($msg:expr) => {
        |e| crate::utils::LocatedError { inner: format!("{:?}", e).into(), file: file!(), line: line!(), column: column!(), addtl_msg: $msg }
    };
}

pub(crate) use eloc;
pub(crate) use eloc_str;


pub async fn get_cache_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
  let mut user_cache_path = dirs::cache_dir().ok_or_else(|| return "No Cache Directory on this operating system!" ).map_err(crate::utils::eloc!())?;
  user_cache_path.push(env!("CARGO_PKG_NAME"));
  tokio::fs::create_dir_all(&user_cache_path).await?;
  Ok(user_cache_path)
}

pub async fn get_cache_file(file_name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut pb = get_cache_dir().await?;
    pb.push(file_name);
    Ok(pb)
}


pub struct DownloadProgressReporter {
    pub max_progress: std::cell::UnsafeCell<std::option::Option<u64>>,
    pub bar: indicatif::ProgressBar,
}

unsafe impl Sync for DownloadProgressReporter { } // Because I said so, our UnsafeCell is just a number in memory

impl DownloadProgressReporter {
    pub fn new() -> Self {
        Self {
            max_progress: None.into(),
            bar: indicatif::ProgressBar::no_length()
        }
    }
}

impl Drop for DownloadProgressReporter {
    fn drop(&mut self) {
        self.bar.finish();
    }
}


impl downloader::progress::Reporter for DownloadProgressReporter {
    fn setup(&self, max_progress: std::option::Option<u64>, message: &str) {
        unsafe { *self.max_progress.get() = max_progress.into(); } // Assigns into a read-only reference; safe because I say the compiler won't optimize through an UnsafeCell
        if let Some(max_progress_val) = max_progress {
            self.bar.set_length(max_progress_val);
        }
    }
    fn progress(&self, current: u64) {
        if current > self.bar.position() {
            let incr_amnt = current - self.bar.position();
            self.bar.inc(incr_amnt);
        }
    }
    fn set_message(&self, message: &str) {

    }
    fn done(&self) {
        self.bar.finish();
    }
}


