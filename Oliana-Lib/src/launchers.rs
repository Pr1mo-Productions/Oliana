
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
    }
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

  pub fn ensure_named_proc_running(&mut self, process_bin_name: String, process_args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut existing_proc_i: Option<usize> = None;
    for i in 0..self.procs.len() {
      if self.procs[i].bin_name == process_bin_name {
        existing_proc_i = Some(i);
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
      let otp = OneTrackedProc {
        proc_track_dir: self.proc_track_dir.clone(),
        bin_name: process_bin_name.to_string(),
        filesystem_bin_path: crate::files::find_newest_mtime_bin_under_folder(&self.expected_bin_directory, &process_bin_name)?,
        filesystem_pid_filepath: self.proc_track_dir.join(format!("{}-pid.txt", process_bin_name)),
      };
      otp.spawn_proc(&process_args, &mut self.spawned_children)?;
      self.procs.push(otp);
    }

    Ok(())
  }
}

// This structure exists to store potentially-expensive-to-lookup items once (eg filesystem_bin_path looked up from bin_name)
pub struct OneTrackedProc {
  pub proc_track_dir: std::path::PathBuf,
  pub bin_name: String,
  pub filesystem_bin_path: std::path::PathBuf,
  pub filesystem_pid_filepath: std::path::PathBuf,
}

impl OneTrackedProc {
  pub fn get_expected_pid(&self) -> Result<Option<u32>, Box<dyn std::error::Error>> {
    if self.filesystem_pid_filepath.exists() {
      let file_content = std::fs::read_to_string(&self.filesystem_pid_filepath).map_err(crate::err::eloc!())?;
      let pid_num = file_content.parse::<u32>().map_err(crate::err::eloc!())?;
      return Ok(Some(pid_num));
    }
    Ok(None)
  }

  pub fn is_running(&self, sinfo: &mut sysinfo::System, spawned_child_holder: &mut Vec<std::process::Child>) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(pid) = self.get_expected_pid().map_err(crate::err::eloc!())? {
      //sinfo.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]), true); // TODO potential future optimization where we don't scan _all_ processes?
      sinfo.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
      // If <pid> is no longer in the process list, it exited!
      // If <pid> is in the process list, return if it's alive or not (which ought to always be true)
      if let Some(process) = sinfo.process(sysinfo::Pid::from_u32(pid)) {
        return Ok(true); // If we have a Process struct after a refresh, <pid> is still running!
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
                true
              },
          }
        });
      }
    }
    Ok(false)
  }

  pub fn spawn_proc(&self, args: &Vec<String>, spawned_child_holder: &mut Vec<std::process::Child>) -> Result<(), Box<dyn std::error::Error>> {
    let child = std::process::Command::new(&self.filesystem_bin_path)
                  .args(args)
                  .spawn().map_err(crate::err::eloc!())?;

    if let Some(dirname) = self.filesystem_pid_filepath.parent() {
      if !dirname.exists() {
        std::fs::create_dir_all(dirname).map_err(crate::err::eloc!())?;
      }
    }

    let pid = child.id();

    let pid_file_content = format!("{pid}");

    std::fs::write(&self.filesystem_pid_filepath, pid_file_content).map_err(crate::err::eloc!())?;

    spawned_child_holder.push(child);
    Ok(())
  }
}

