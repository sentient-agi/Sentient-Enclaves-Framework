use anyhow::Result;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use crate::protocol::ProcessInfo;

/// Parse /proc/[pid]/stat file
fn parse_proc_stat(pid: i32) -> Result<(String, String, i32, u64, u64)> {
    let path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(&path)?;

    // Parse stat file format
    // pid (comm) state ppid ...
    let start = content.find('(').unwrap_or(0);
    let end = content.rfind(')').unwrap_or(content.len());

    let name = if start < end {
        content[start + 1..end].to_string()
    } else {
        String::from("unknown")
    };

    let after_name = &content[end + 2..];
    let fields: Vec<&str> = after_name.split_whitespace().collect();

    let state = fields.get(0).unwrap_or(&"?").to_string();
    let ppid = fields.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let utime = fields.get(11).and_then(|s| s.parse().ok()).unwrap_or(0);
    let stime = fields.get(12).and_then(|s| s.parse().ok()).unwrap_or(0);
    let starttime = fields.get(19).and_then(|s| s.parse().ok()).unwrap_or(0);

    Ok((name, state, ppid, utime + stime, starttime))
}

/// Parse /proc/[pid]/cmdline
fn parse_proc_cmdline(pid: i32) -> String {
    let path = format!("/proc/{}/cmdline", pid);
    match fs::read(&path) {
        Ok(bytes) => {
            let s = String::from_utf8_lossy(&bytes);
            s.replace('\0', " ").trim().to_string()
        }
        Err(_) => String::new(),
    }
}

/// Get memory usage from /proc/[pid]/status
fn get_memory_kb(pid: i32) -> u64 {
    let path = format!("/proc/{}/status", pid);
    if let Ok(content) = fs::read_to_string(&path) {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    return value.parse().unwrap_or(0);
                }
            }
        }
    }
    0
}

/// Calculate CPU percentage
fn calculate_cpu_percent(total_time: u64, start_time: u64, uptime_secs: u64) -> f32 {
    let hertz = 100; // CONFIG_HZ, usually 100
    let seconds = uptime_secs.saturating_sub(start_time / hertz);
    if seconds > 0 {
        ((total_time * 100) as f32) / ((seconds * hertz) as f32)
    } else {
        0.0
    }
}

/// Get system uptime in seconds
fn get_uptime_secs() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/uptime") {
        if let Some(uptime_str) = content.split_whitespace().next() {
            return uptime_str.parse::<f64>().unwrap_or(0.0) as u64;
        }
    }
    0
}

/// List all processes
pub fn list_processes(service_pids: &HashMap<String, i32>) -> Vec<ProcessInfo> {
    let mut processes = Vec::new();
    let uptime_secs = get_uptime_secs();

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                if let Ok(pid) = file_name.parse::<i32>() {
                    if let Ok(info) = get_process_info(pid, uptime_secs, service_pids) {
                        processes.push(info);
                    }
                }
            }
        }
    }

    // Sort by PID
    processes.sort_by_key(|p| p.pid);
    processes
}

/// Get information about a specific process
pub fn get_process_info(
    pid: i32,
    uptime_secs: u64,
    service_pids: &HashMap<String, i32>,
) -> Result<ProcessInfo> {
    let (name, state, ppid, total_time, start_time) = parse_proc_stat(pid)?;
    let cmdline = parse_proc_cmdline(pid);
    let memory_kb = get_memory_kb(pid);
    let cpu_percent = calculate_cpu_percent(total_time, start_time, uptime_secs);

    // Check if this process is managed by init
    let (managed, service_name) = service_pids
        .iter()
        .find(|(_, &p)| p == pid)
        .map(|(name, _)| (true, Some(name.clone())))
        .unwrap_or((false, None));

    Ok(ProcessInfo {
        pid,
        ppid,
        name: name.clone(),
        cmdline: if cmdline.is_empty() {
            format!("[{}]", name)
        } else {
            cmdline
        },
        state,
        cpu_percent,
        memory_kb,
        start_time,
        managed,
        service_name,
    })
}

/// Send signal to process
pub fn signal_process(pid: i32, signal: Signal) -> Result<()> {
    kill(Pid::from_raw(pid), signal)?;
    Ok(())
}

/// Start a new process (not managed by init)
pub fn start_process(
    command: &str,
    args: &[String],
    env: &[String],
) -> Result<i32> {
    use nix::unistd::{fork, ForkResult};
    use std::ffi::CString;

    match unsafe { fork()? } {
        ForkResult::Parent { child } => {
            Ok(child.as_raw())
        }
        ForkResult::Child => {
            // Build argument list
            let mut full_args = vec![command.to_string()];
            full_args.extend_from_slice(args);

            let argv_c: Vec<CString> = full_args
                .iter()
                .filter_map(|s| CString::new(s.as_str()).ok())
                .collect();

            let envp_c: Vec<CString> = env
                .iter()
                .filter_map(|s| CString::new(s.as_str()).ok())
                .collect();

            let mut argv_ptrs: Vec<*const libc::c_char> =
                argv_c.iter().map(|s| s.as_ptr()).collect();
            argv_ptrs.push(std::ptr::null());

            let mut envp_ptrs: Vec<*const libc::c_char> =
                envp_c.iter().map(|s| s.as_ptr()).collect();
            envp_ptrs.push(std::ptr::null());

            unsafe {
                libc::execvpe(argv_ptrs[0], argv_ptrs.as_ptr(), envp_ptrs.as_ptr());
            }

            std::process::exit(1);
        }
    }
}
