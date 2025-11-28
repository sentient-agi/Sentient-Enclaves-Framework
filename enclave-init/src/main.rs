use anyhow::{bail, Context, Result};
use nix::errno::Errno;
use nix::mount::{mount, MsFlags};
use nix::sys::signal::{sigprocmask, SigSet, SigmaskHow, Signal};
use nix::sys::socket::{connect, socket, AddressFamily, SockFlag, SockType, VsockAddr};
use nix::sys::stat::{makedev, mknod, Mode, SFlag};
use nix::sys::wait::{wait, WaitStatus};
use nix::unistd::{chdir, chroot, close, fork, read, setsid, setpgid, symlinkat, unlink, write, ForkResult, Pid};
use std::ffi::{CString, OsStr};
use std::fs::{self, create_dir, File};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::process;

// Constants
const DEFAULT_PATH_ENV: &str = "PATH=/sbin:/usr/sbin:/bin:/usr/bin";
const NSM_PATH: &str = "nsm.ko";
const VSOCK_PORT: u32 = 9000;
const VSOCK_CID: u32 = 3;
const HEART_BEAT: u8 = 0xB7;

const DEFAULT_ENVP: &[&str] = &[DEFAULT_PATH_ENV];
const DEFAULT_ARGV: &[&str] = &["sh"];

// Error handling helpers
fn warn(msg: &str) {
    eprintln!("{}: {}", msg, std::io::Error::last_os_error());
}

fn warn2(msg1: &str, msg2: &str) {
    eprint!("{}: ", msg1);
    warn(msg2);
}

fn die(msg: &str) -> ! {
    warn(msg);
    process::exit(Errno::last() as i32);
}

fn die2(msg1: &str, msg2: &str) -> ! {
    warn2(msg1, msg2);
    process::exit(Errno::last() as i32);
}

macro_rules! die_on {
    ($cond:expr, $msg:expr) => {
        if $cond {
            die($msg);
        }
    };
}

// Operation types
#[derive(Debug)]
enum InitOp {
    Mount {
        source: &'static str,
        target: &'static str,
        fstype: &'static str,
        flags: MsFlags,
        data: Option<&'static str>,
    },
    Mkdir {
        path: &'static str,
        mode: u32,
    },
    Mknod {
        path: &'static str,
        mode: Mode,
        major: u64,
        minor: u64,
    },
    Symlink {
        linkpath: &'static str,
        target: &'static str,
    },
}

// List of initialization operations
// const OPS: std::sync::LazyLock<Vec<InitOp>> = std::sync::LazyLock::new(|| vec![
static OPS: std::sync::LazyLock<Vec<InitOp>> = std::sync::LazyLock::new(|| vec![
    // mount /proc (which should already exist)
    InitOp::Mount {
        source: "proc",
        target: "/proc",
        fstype: "proc",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    // add symlinks in /dev (which is already mounted)
    InitOp::Symlink {
        linkpath: "/dev/fd",
        target: "/proc/self/fd",
    },
    InitOp::Symlink {
        linkpath: "/dev/stdin",
        target: "/proc/self/fd/0",
    },
    InitOp::Symlink {
        linkpath: "/dev/stdout",
        target: "/proc/self/fd/1",
    },
    InitOp::Symlink {
        linkpath: "/dev/stderr",
        target: "/proc/self/fd/2",
    },
    // mount tmpfs on /run and /tmp (which should already exist)
    InitOp::Mount {
        source: "tmpfs",
        target: "/run",
        fstype: "tmpfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: Some("mode=0755"),
    },
    InitOp::Mount {
        source: "tmpfs",
        target: "/tmp",
        fstype: "tmpfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    // mount shm and devpts
    InitOp::Mkdir {
        path: "/dev/shm",
        mode: 0o755,
    },
    InitOp::Mount {
        source: "shm",
        target: "/dev/shm",
        fstype: "tmpfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    InitOp::Mkdir {
        path: "/dev/pts",
        mode: 0o755,
    },
    InitOp::Mount {
        source: "devpts",
        target: "/dev/pts",
        fstype: "devpts",
        flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    // mount /sys (which should already exist)
    InitOp::Mount {
        source: "sysfs",
        target: "/sys",
        fstype: "sysfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    InitOp::Mount {
        source: "cgroup_root",
        target: "/sys/fs/cgroup",
        fstype: "tmpfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: Some("mode=0755"),
    },
]);

fn init_dev() {
    let result = mount(
        Some("dev"),
        "/dev",
        Some("devtmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        None::<&str>,
    );

    if let Err(e) = result {
        warn2("mount", "/dev");
        // /dev will be already mounted if devtmpfs.mount = 1 on the kernel
        // command line or CONFIG_DEVTMPFS_MOUNT is set. Do not consider this
        // an error.
        if e != Errno::EBUSY {
            process::exit(e as i32);
        }
    }
}

fn init_fs(ops: &[InitOp]) {
    for op in ops {
        match op {
            InitOp::Mount {
                source,
                target,
                fstype,
                flags,
                data,
            } => {
                if let Err(_) = mount(Some(*source), *target, Some(*fstype), *flags, *data) {
                    die2("mount", target);
                }
            }
            InitOp::Mkdir { path, mode } => {
                if let Err(e) = create_dir(*path) {
                    if e.kind() != std::io::ErrorKind::AlreadyExists {
                        warn2("mkdir", path);
                        process::exit(Errno::last() as i32);
                    }
                }
                // Set permissions
                let _ = fs::set_permissions(*path, std::fs::Permissions::from_mode(*mode));
            }
            InitOp::Mknod { path, mode, major, minor } => {
                let dev = makedev(*major, *minor);
                if let Err(e) = mknod(Path::new(path), SFlag::from_bits_truncate(mode.bits()), *mode, dev) {
                    if e != Errno::EEXIST {
                        warn2("mknod", path);
                        process::exit(e as i32);
                    }
                }
            }
            InitOp::Symlink { linkpath, target } => {
                if let Err(e) = symlinkat(*target, None, *linkpath) {
                    if e != Errno::EEXIST {
                        warn2("symlink", linkpath);
                        process::exit(e as i32);
                    }
                }
            }
        }
    }
}

fn init_cgroups() {
    let fpath = "/proc/cgroups";
    let file = match File::open(fpath) {
        Ok(f) => f,
        Err(_) => die2("fopen", fpath),
    };

    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Skip the first line
    let _ = lines.next();

    for line in lines {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 4 {
            continue;
        }

        let name = parts[0];
        let enabled = parts[3].parse::<i32>().unwrap_or(0);

        if enabled != 0 {
            let path = format!("/sys/fs/cgroup/{}", name);
            if let Err(_) = create_dir(&path) {
                die2("mkdir", &path);
            }

            if let Err(_) = mount(
                Some(name),
                path.as_str(),
                Some("cgroup"),
                MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
                Some(name),
            ) {
                die2("mount", &path);
            }
        }
    }
}

fn init_console() {
    // init process needs to set up a tty for the container
    let console_path = "/dev/console";

    unsafe {
        let mode_r = CString::new("r").unwrap();
        let mode_w = CString::new("w").unwrap();
        let path = CString::new(console_path).unwrap();

        die_on!(
            libc::freopen(path.as_ptr(), mode_r.as_ptr(), libc_stdhandle::stdin()).is_null(),
            "freopen failed for stdin"
        );
        die_on!(
            libc::freopen(path.as_ptr(), mode_w.as_ptr(), libc_stdhandle::stdout()).is_null(),
            "freopen failed for stdout"
        );
        die_on!(
            libc::freopen(path.as_ptr(), mode_w.as_ptr(), libc_stdhandle::stderr()).is_null(),
            "freopen failed for stderr"
        );
    }
}

// Helper module to get stdio handles
mod libc_stdhandle {
    use std::os::raw::c_int;

    extern "C" {
        #[link_name = "stdin"]
        static stdin_ptr: *mut libc::FILE;
        #[link_name = "stdout"]
        static stdout_ptr: *mut libc::FILE;
        #[link_name = "stderr"]
        static stderr_ptr: *mut libc::FILE;
    }

    pub unsafe fn stdin() -> *mut libc::FILE {
        stdin_ptr
    }

    pub unsafe fn stdout() -> *mut libc::FILE {
        stdout_ptr
    }

    pub unsafe fn stderr() -> *mut libc::FILE {
        stderr_ptr
    }
}

fn launch(argv: Option<Vec<String>>, envp: Option<Vec<String>>) -> Pid {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            return child;
        }
        Ok(ForkResult::Child) => {
            let argv = argv.unwrap_or_else(|| DEFAULT_ARGV.iter().map(|s| s.to_string()).collect());
            let mut envp = envp.unwrap_or_else(|| DEFAULT_ENVP.iter().map(|s| s.to_string()).collect());

            // Unblock signals before execing
            let set = SigSet::all();
            let _ = sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&set), None);

            // Create a session and process group
            let _ = setsid();
            let _ = setpgid(Pid::from_raw(0), Pid::from_raw(0));

            // Set PATH environment variable
            envp.push(DEFAULT_PATH_ENV.to_string());

            // Convert to CStrings
            let argv_c: Vec<CString> = argv
                .iter()
                .map(|s| CString::new(s.as_str()).unwrap())
                .collect();
            let envp_c: Vec<CString> = envp
                .iter()
                .map(|s| CString::new(s.as_str()).unwrap())
                .collect();

            // Convert to raw pointers
            let mut argv_ptrs: Vec<*const libc::c_char> = argv_c.iter().map(|s| s.as_ptr()).collect();
            argv_ptrs.push(std::ptr::null());

            let mut envp_ptrs: Vec<*const libc::c_char> = envp_c.iter().map(|s| s.as_ptr()).collect();
            envp_ptrs.push(std::ptr::null());

            // Execute
            unsafe {
                libc::execvpe(argv_ptrs[0], argv_ptrs.as_ptr(), envp_ptrs.as_ptr());
            }

            die2("execvpe", &argv[0]);
        }
        Err(_) => {
            die("fork");
        }
    }
}

fn reap_until(until_pid: Pid) -> i32 {
    loop {
        match wait() {
            Ok(WaitStatus::Exited(pid, status)) => {
                if pid == until_pid {
                    if status != 0 {
                        eprintln!("child exited with error");
                    }
                    return status;
                }
            }
            Ok(WaitStatus::Signaled(pid, signal, _)) => {
                if pid == until_pid {
                    eprintln!("child exited by signal");
                    return 128 + signal as i32;
                }
            }
            Err(_) => {
                die("wait");
            }
            _ => {}
        }
    }
}

fn read_config(path: &str) -> Option<Vec<String>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            warn(&format!("Could not open {} file", path));
            return None;
        }
    };

    let reader = BufReader::new(file);
    let mut result = Vec::new();

    for line in reader.lines() {
        if let Ok(mut line) = line {
            if line.ends_with('\n') {
                line.pop();
            }
            result.push(line);
        }
    }

    Some(result)
}

fn enclave_ready() {
    let socket_fd = match socket(
        AddressFamily::Vsock,
        SockType::Stream,
        SockFlag::empty(),
        None,
    ) {
        Ok(fd) => fd,
        Err(_) => die("socket"),
    };

    let addr = VsockAddr::new(VSOCK_CID, VSOCK_PORT);

    die_on!(connect(socket_fd, &addr).is_err(), "connect");

    let buf = [HEART_BEAT];
    die_on!(write(socket_fd, &buf).unwrap_or(0) != 1, "write heartbeat");

    let mut buf_read = [0u8; 1];
    die_on!(read(socket_fd, &mut buf_read).unwrap_or(0) != 1, "read heartbeat");
    die_on!(buf_read[0] != HEART_BEAT, "received wrong heartbeat");
    die_on!(close(socket_fd).is_err(), "close");
}

fn init_nsm_driver() {
    use std::os::unix::io::IntoRawFd;

    let fd = match File::open(NSM_PATH) {
        Ok(f) => f.into_raw_fd(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return;
        }
        Err(_) => die("failed to open nsm fd"),
    };

    let params = CString::new("").unwrap();
    let rc = unsafe { libc::syscall(libc::SYS_finit_module, fd, params.as_ptr(), 0) };
    die_on!(rc < 0, "failed to insert nsm driver");

    die_on!(unsafe { libc::close(fd) } < 0, "close nsm fd");

    if let Err(_) = unlink(NSM_PATH) {
        warn("Could not unlink nsm.ko");
    }
}

fn main() {
    // Block all signals in init. SIGCHLD will still cause wait() to return.
    let set = SigSet::all();
    let _ = sigprocmask(SigmaskHow::SIG_BLOCK, Some(&set), None);

    // Set up the minimal dependencies to start a container
    // Init /dev and start /dev/console for early debugging
    init_dev();
    init_console();

    // Insert the Nitro Secure Module driver
    init_nsm_driver();

    // Signal nitro-cli that the enclave has started
    enclave_ready();

    // env should be an array of "VAR1=string1", "VAR2=string2", ...
    let env = read_config("/env");
    // cmd should be an array of "command", "param1", "param2", ...
    let cmd = read_config("/cmd");

    let _ = unlink("/env");
    let _ = unlink("/cmd");

    // Turn /rootfs into a mount point so it can be used with mount --move
    die_on!(
        mount(
            Some("/rootfs"),
            "/rootfs",
            None::<&str>,
            MsFlags::MS_BIND,
            None::<&str>
        )
        .is_err(),
        "mount --bind /rootfs /rootfs"
    );

    die_on!(chdir("/rootfs").is_err(), "chdir /rootfs");

    // Change the root directory of the mount namespace to the root directory
    // by overmounting / with /rootfs
    die_on!(
        mount(Some("."), "/", None::<&str>, MsFlags::MS_MOVE, None::<&str>).is_err(),
        "mount --move . /"
    );

    die_on!(chroot(".").is_err(), "chroot .");
    die_on!(chdir("/").is_err(), "chdir /");

    // At this point, we need to make sure the container /dev is initialized as well.
    init_dev();
    init_fs(&OPS);
    init_cgroups();

    let pid = launch(cmd, env);

    // Reap until the initial child process dies.
    let exit_code = reap_until(pid);

    unsafe {
        libc::reboot(libc::RB_AUTOBOOT);
    }

    process::exit(exit_code);
}

// Required for setting file permissions from mode
trait FromMode {
    fn from_mode(mode: u32) -> Self;
}

impl FromMode for std::fs::Permissions {
    fn from_mode(mode: u32) -> Self {
        use std::os::unix::fs::PermissionsExt;
        <std::fs::Permissions as PermissionsExt>::from_mode(mode)
    }
}
