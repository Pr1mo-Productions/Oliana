
use crate as oliana_lib;

// This structure is responsible for watching over the
// Oliana-Text and Oliana-Text subprocesses and providing accessors to their
// outputs over a directory structure.
// All state will be tracked under proc_track_dir.
// This structure should ONLY be used by server-side processes which know they have a GPU attached.
pub struct TrackedProcs {
  pub proc_track_dir: std::path::PathBuf,
  pub expected_bin_directory: std::path::PathBuf,
  pub procs: Vec<OneTrackedProc>,
  pub tracked_proc_args: Vec<(String, Vec<String>)>,
  pub sinfo: sysinfo::System,
  pub spawned_children: Vec<std::process::Child>,
  pub procs_should_be_stopped: bool,
}

impl TrackedProcs {
  pub fn new(proc_track_dir: impl Into<std::path::PathBuf>, expected_bin_directory: impl Into<std::path::PathBuf>) -> Self {
    Self {
      proc_track_dir: proc_track_dir.into(),
      expected_bin_directory: expected_bin_directory.into(),
      procs: Vec::with_capacity(8),
      tracked_proc_args: Vec::with_capacity(8),
      sinfo: sysinfo::System::new(),
      spawned_children: Vec::with_capacity(32),
      procs_should_be_stopped: false,
    }
  }

  pub fn new_from_env() -> Result<Self, Box<dyn std::error::Error>> {
    Ok(Self {
      proc_track_dir: std::env::var("OLIANA_TRACKED_PROC_DIR")?.into(),
      expected_bin_directory: std::env::var("OLIANA_BIN_DIR")?.into(),
      procs: Vec::with_capacity(8),
      tracked_proc_args: Vec::with_capacity(8),
      sinfo: sysinfo::System::new(),
      spawned_children: Vec::with_capacity(32),
      procs_should_be_stopped: false,
    })
  }

  pub fn register_tracked_proc(&mut self, process_bin_name: &str, process_args: &[&str]) {
    let mut owned_p_args = Vec::with_capacity(process_args.len());
    for arg in process_args {
      owned_p_args.push(arg.to_string());
    }
    self.tracked_proc_args.push(
      (process_bin_name.to_string(), owned_p_args)
    );
  }

  pub fn ensure_registered_procs_running(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..self.tracked_proc_args.len() {
      self.ensure_named_proc_running(self.tracked_proc_args[i].0.clone(), self.tracked_proc_args[i].1.clone())?; // TODO engineer those .clone()s out of here!
    }
    Ok(())
  }

  pub fn resume_sigstop_procs(&self, resume_for_duration: std::time::Duration) -> Result<(), Box<dyn std::error::Error>> {
    if self.procs_should_be_stopped {
      // TODO
      #[cfg(target_os = "linux")]
      {
        if let Err(e) = self.send_signal_to_children(nix::sys::signal::Signal::SIGCONT) {
          eprintln!("{}:{} {:?}", file!(), line!(), e);
        }
        std::thread::sleep(resume_for_duration);
        if let Err(e) = self.send_signal_to_children(nix::sys::signal::Signal::SIGSTOP) {
          eprintln!("{}:{} {:?}", file!(), line!(), e);
        }
      }
    }
    Ok(())
  }

  pub fn set_procs_should_be_stopped(&mut self, should_be_stopped: bool) {
    if ! should_be_stopped && self.procs_should_be_stopped {
      // Send sig-cont to everyone!
      #[cfg(target_os = "linux")]
      {
        if let Err(e) = self.send_signal_to_children(nix::sys::signal::Signal::SIGCONT) {
          eprintln!("{}:{} {:?}", file!(), line!(), e);
        }
      }
    }
    self.procs_should_be_stopped = should_be_stopped;
  }

  #[cfg(target_os = "linux")]
  pub fn send_signal_to_children(&self, signal: impl Into<nix::sys::signal::Signal>) -> Result<(), Box<dyn std::error::Error>> {
    let signal = signal.into();
    for i in 0..self.procs.len() {
      if let Some(pid) = self.procs[i].get_last_expected_pid_fast() {
        if let Err(e) = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), signal) {
          eprintln!("{}:{} {:?}", file!(), line!(), e);
        }
      }
      else {
        // Could not get a FAST pid, so for correctness we'll go ALL THE WAY to the filesystem for it -_-
        if let Ok(Some(pid)) = self.procs[i].get_expected_pid() {
          if let Err(e) = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), signal) {
            eprintln!("{}:{} {:?}", file!(), line!(), e);
          }
        }
      }
    }
    Ok(())
  }

  // This is called periodically & is responsible for calling .update_proc_output_txt_from_files() on running processes; it has the mutable access to the data to do that.
  pub fn ensure_named_proc_running(&mut self, process_bin_name: String, process_args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut existing_proc_i: Option<usize> = None;
    for i in 0..self.procs.len() {
      if self.procs[i].bin_name == process_bin_name {
        existing_proc_i = Some(i);
        if let Err(e) = self.procs[i].update_proc_output_txt_from_files() {
          eprintln!("{}:{} {}", file!(), line!(), e);
        }
        // Found the process, is it running?
      }
    }
    if let Some(i) = existing_proc_i {
      if !(self.procs[i].is_running(&mut self.sinfo, &mut self.spawned_children)?) {
        self.procs[i].spawn_proc(&process_args, &mut self.spawned_children)?;
      }
    }
    else {
      // Must create a new tracked process & spawn it
      let mut otp = OneTrackedProc {
        proc_track_dir: self.proc_track_dir.clone(),
        bin_name: process_bin_name.to_string(),
        filesystem_bin_path: crate::files::find_newest_mtime_bin_under_folder(&self.expected_bin_directory, &process_bin_name)?,
        filesystem_pid_filepath: self.proc_track_dir.join(format!("{}-pid.txt", process_bin_name)),
        filesystem_stdout_filepath: self.proc_track_dir.join(format!("{}-stdout.txt", process_bin_name)),
        filesystem_stdout_read_bytes: 0,
        filesystem_stderr_filepath: self.proc_track_dir.join(format!("{}-stderr.txt", process_bin_name)),
        filesystem_stderr_read_bytes: 0,
        proc_restart_count: 0,
        proc_output_txt: String::new(),
        last_expected_pid: std::sync::RwLock::new(None)
      };
      otp.spawn_proc(&process_args, &mut self.spawned_children)?;
      self.procs.push(otp);
    }

    Ok(())
  }

  pub fn get_proc_restart_counts(&self) -> std::collections::HashMap::<String, u32> {
    let mut hm = std::collections::HashMap::new();
    for i in 0..self.procs.len() {
      hm.insert(self.procs[i].bin_name.clone(), self.procs[i].proc_restart_count);
    }
    hm
  }

  pub fn get_proc_outputs(&self) -> std::collections::HashMap::<String, String> {
    let mut hm = std::collections::HashMap::new();
    for i in 0..self.procs.len() {
      hm.insert(self.procs[i].bin_name.clone(), self.procs[i].proc_output_txt.clone());
    }
    hm
  }

}

// This structure exists to store potentially-expensive-to-lookup items once (eg filesystem_bin_path looked up from bin_name)
pub struct OneTrackedProc {
  pub proc_track_dir: std::path::PathBuf,
  pub bin_name: String,
  pub filesystem_bin_path: std::path::PathBuf,
  pub filesystem_pid_filepath: std::path::PathBuf,
  pub filesystem_stdout_filepath: std::path::PathBuf,
  pub filesystem_stdout_read_bytes: usize,
  pub filesystem_stderr_filepath: std::path::PathBuf,
  pub filesystem_stderr_read_bytes: usize,
  pub proc_restart_count: u32,
  pub proc_output_txt: String,
  pub last_expected_pid: std::sync::RwLock::<Option<u32>>,
}

impl OneTrackedProc {
  pub fn get_expected_pid(&self) -> Result<Option<u32>, Box<dyn std::error::Error>> {
    if self.filesystem_pid_filepath.exists() {
      let file_content = std::fs::read_to_string(&self.filesystem_pid_filepath).map_err(crate::err::eloc!())?;
      let pid_num = file_content.parse::<u32>().map_err(crate::err::eloc!())?;
      if let Ok(mut write_lock) = self.last_expected_pid.write() {
        *write_lock = Some(pid_num);
      }
      return Ok(Some(pid_num));
    }
    Ok(None)
  }

  pub fn get_last_expected_pid_fast(&self) -> Option<u32> {
    let mut result = None;
    if let Ok(read_lock) = self.last_expected_pid.read() {
      if let Some(val) = *read_lock {
        result = Some(val);
      }
    }
    result
  }

  pub fn is_running(&self, sinfo: &mut sysinfo::System, spawned_child_holder: &mut Vec<std::process::Child>) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(pid) = self.get_expected_pid().map_err(crate::err::eloc!())? {
      //sinfo.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]), true); // TODO potential future optimization where we don't scan _all_ processes?
      sinfo.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
      // If <pid> is no longer in the process list, it exited!
      // If <pid> is in the process list, return if it's alive or not (which ought to always be true)
      if let Some(process) = sinfo.process(sysinfo::Pid::from_u32(pid)) {
        // Process exists, should we reap it?
        match process.status() {
          sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead => {
            // Process _just_ exited, therefore it is _not_ running!

            // Retain all children which are NOT this process.
            spawned_child_holder.retain_mut(|c| {
              if c.id() != pid {
                true
              }
              else {
                // Reap the child process
                if let Err(e) = c.wait() {
                  eprintln!("{:?}", e);
                }
                false
              }
            });

            return Ok(false);
          }
          unused => {
            return Ok(true); // <pid> is still running!
          }
        }
      }
      else {
        // If we thought we had a PID and no longer have it, re-scan processes in spawned_child_holder and remove them
        spawned_child_holder.retain_mut(|c| {
          match c.try_wait() {
              Ok(Some(status)) => false, /* remove because exited w/ a code */
              Ok(None) => {
                  true /* has yet to exit, keep reference */
              }
              Err(e) => {
                /* misc OS error, keep reference */
                eprintln!("Within spawned_child_holder.retain_mut: {:?}", e);
                true
              },
          }
        });
      }
    }
    Ok(false)
  }

  pub fn update_proc_output_txt_from_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;

    // We simply append here, spawn_proc.() will place a "================ PID {} ================" banner across process re-starts.
    // If length is ever > 32kb, we remove the first 8kb chunk from the beginning of the in-memory string
    if let Ok(current_pid_stdout) = std::fs::read_to_string(&self.filesystem_stdout_filepath) {
      if current_pid_stdout.len() > self.filesystem_stdout_read_bytes {
        let new_stdout = &current_pid_stdout[self.filesystem_stdout_read_bytes..];
        self.proc_output_txt.push_str(new_stdout);
        self.filesystem_stdout_read_bytes += current_pid_stdout.len() - self.filesystem_stdout_read_bytes;
        print!("{}", new_stdout);
        if let Err(e) = std::io::stdout().flush() {
            println!("{}:{} {}", file!(), line!(), e);
        }
      }
    }
    if let Ok(current_pid_stderr) = std::fs::read_to_string(&self.filesystem_stderr_filepath) {
      if current_pid_stderr.len() > self.filesystem_stderr_read_bytes {
        let new_stderr = &current_pid_stderr[self.filesystem_stderr_read_bytes..];
        self.proc_output_txt.push_str(new_stderr);
        self.filesystem_stderr_read_bytes += current_pid_stderr.len() - self.filesystem_stderr_read_bytes;
        eprint!("{}", new_stderr);
        if let Err(e) = std::io::stdout().flush() {
            println!("{}:{} {}", file!(), line!(), e);
        }
      }
    }

    if self.proc_output_txt.len() > 32 * 1024 {
      let mut closest_char_idx = 8192;
      for (char_idx, _c) in self.proc_output_txt.char_indices() {
        if char_idx > closest_char_idx {
          closest_char_idx = char_idx;
          break;
        }
      }
      self.proc_output_txt = self.proc_output_txt[closest_char_idx..].to_string();
    }

    Ok(())
  }

  pub fn spawn_proc(&mut self, args: &Vec<String>, spawned_child_holder: &mut Vec<std::process::Child>) -> Result<(), Box<dyn std::error::Error>> {

    let debug_process_line = format!("{} {}", self.filesystem_bin_path.display(), args.join(" "));
    eprintln!("Spawning the process: {debug_process_line}");

    if self.filesystem_stdout_filepath.exists() {
      if let Err(e) = std::fs::remove_file(&self.filesystem_stdout_filepath) {
        eprintln!("{}:{} {}", file!(), line!(), e);
      }
    }
    if self.filesystem_stderr_filepath.exists() {
      if let Err(e) = std::fs::remove_file(&self.filesystem_stderr_filepath) {
        eprintln!("{}:{} {}", file!(), line!(), e);
      }
    }

    let child_stdout = std::fs::File::create(&self.filesystem_stdout_filepath)?; // We will have to remember to regularly read from these and eprintln!() + write to self.proc_output_txt
    let child_stderr = std::fs::File::create(&self.filesystem_stderr_filepath)?;

    let child = std::process::Command::new(&self.filesystem_bin_path)
                  .args(args)
                  .stdin(std::process::Stdio::null())
                  .stdout(child_stdout)
                  .stderr(child_stderr)
                  .spawn().map_err(crate::err::eloc!())?;

    if let Some(dirname) = self.filesystem_pid_filepath.parent() {
      if !dirname.exists() {
        std::fs::create_dir_all(dirname).map_err(crate::err::eloc!())?;
      }
    }

    let pid = child.id();

    if let Ok(mut write_lock) = self.last_expected_pid.write() {
      *write_lock = Some(pid);
    }

    let pid_file_content = format!("{pid}");

    eprintln!("Writing PID ({}) of new {} to {}", &pid_file_content[..], self.filesystem_bin_path.display(), self.filesystem_pid_filepath.display());

    std::fs::write(&self.filesystem_pid_filepath, pid_file_content).map_err(crate::err::eloc!())?;

    self.proc_restart_count += 1;
    self.proc_output_txt.push_str(&format!("================ PID {pid} ================\n"));
    self.filesystem_stdout_read_bytes = 0;
    self.filesystem_stderr_read_bytes = 0;

    spawned_child_holder.push(child);

    Ok(())
  }
}

