
use crate as oliana_lib; // This helps our crate::err::eloc!() leak state via a struct

pub async fn existinate(
  local_file_path: impl Into<std::path::PathBuf>,
  remote_download_url: &str
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
  let local_file_path = local_file_path.into();

  if !tokio::fs::try_exists(&local_file_path).await? {
    eprintln!("Downloading {} to {}", remote_download_url, &local_file_path.to_string_lossy() );
    if remote_download_url.len() < 1 {
      return Err(format!("The file {:?} does not exist and no URL was passed to download it!", &local_file_path).into());
    }

    let mut downloader = downloader::Downloader::builder()
          .download_folder( local_file_path.parent().ok_or_else(|| return "No Parent Directory for passed file to be downloaded!" ).map_err(crate::err::eloc!())? )
          .parallel_requests(2)
          .build()?;
    let dl_file_name_osstr = local_file_path.file_name().ok_or_else(|| return "No File Name for passed file to be downloaded!" ).map_err(crate::err::eloc!())?;
    let dl_file_name_string = dl_file_name_osstr.to_string_lossy().into_owned();

    let dl = downloader::Download::new(remote_download_url)
                .file_name( &std::path::Path::new( &dl_file_name_string ) )
                .progress(std::sync::Arc::new(
                  DownloadProgressReporter::new()
                ));

    let _result = downloader.async_download(&[dl]).await?;

  }
  else {
    eprintln!("Found already-downloaded file {}", &local_file_path.to_string_lossy() );
  }

  Ok(local_file_path)
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




pub async fn get_cache_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
  let mut user_cache_path = dirs::cache_dir().ok_or_else(|| return "No Cache Directory on this operating system!" ).map_err(crate::err::eloc!())?;
  user_cache_path.push(env!("CARGO_PKG_NAME"));
  tokio::fs::create_dir_all(&user_cache_path).await?;
  Ok(user_cache_path)
}

pub async fn get_cache_file(file_name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let mut pb = get_cache_dir().await?;
    pb.push(file_name);
    Ok(pb)
}

#[cfg(target_os="windows")]
pub fn append_os_extention_to_bin(bin_name: &str) -> String {
    if bin_name.ends_with(".exe") || bin_name.ends_with(".EXE") {
        return bin_name.to_string();
    }
    return format!("{bin_name}.exe");
}
#[cfg(not(target_os="windows"))]
pub fn append_os_extention_to_bin(bin_name: &str) -> String {
    return bin_name.to_string();
}


pub fn find_newest_mtime_bin_under_folder(folder: &std::path::Path, bin_name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let bin_name = append_os_extention_to_bin(bin_name);

    let bin_name_osstr: &std::ffi::OsStr = bin_name.as_ref();

    let mut newest_path: Option<std::path::PathBuf> = None;
    let mut newest_mtime: filetime::FileTime = filetime::FileTime::zero();

    for entry in walkdir::WalkDir::new(folder) {
        if let Ok(entry) = entry {
            let epath = entry.path();
            if !epath.is_file() {
                continue; // Skip folder
            }
            if let Some(file_name_osstr) = epath.file_name() {
                if file_name_osstr != bin_name_osstr {
                    continue; // Skip things not of same name
                }
            }
            else {
                continue; // Skip things w/o a name at all
            }

            match std::fs::metadata(epath) {
                Ok(metadata) => {
                    let mtime = filetime::FileTime::from_last_modification_time(&metadata);
                    if mtime > newest_mtime {
                        newest_path = Some(epath.to_path_buf());
                        newest_mtime = mtime;
                    }
                }
                Err(e) => {
                    eprintln!("{}:{} {:?}", file!(), line!(), e);
                }
            }

        }
    }

    if let Some(newest_pb) = newest_path {
        return Ok(newest_pb);
    }
    else {
        return Err(format!("Failed to lookup the program {bin_name:?} under {folder:?}").into())
    }
}





