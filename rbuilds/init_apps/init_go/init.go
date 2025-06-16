//go:build linux
// +build linux

package main

import (
	"bufio"
	"errors"
	"fmt"
	"golang.org/x/sys/unix"
	"io"
	"os"
	"syscall"
	"unsafe"
)

const (
	DEFAULT_PATH_ENV = "PATH=/sbin:/usr/sbin:/bin:/usr/bin"
	NSM_PATH         = "nsm.ko"
	TIMEOUT          = 20000 // millis
	VSOCK_PORT       = 9000
	VSOCK_CID        = 3
	HEART_BEAT       = 0xB7
	RB_AUTOBOOT      = 0x1234567
)

type Mount struct {
	source, target, mType string
	flags                 uintptr
	data                  string
}

type Mkdir struct {
	path string
	mode uint32
}

type Symlink struct {
	linkpath, target string
}

type InitOp interface {
	doOrDie()
}

func newErr(err error, msg string) error {
	return errors.Join(errors.New(msg), err)
}

var ops = []InitOp{
	// mount /proc (which should already exist)
	&Mount{"proc", "/proc", "proc", syscall.MS_NODEV | syscall.MS_NOSUID | syscall.MS_NOEXEC, ""},

	// add symlinks in /dev (which is already mounted)
	&Symlink{"/dev/fd", "/proc/self/fd"},
	&Symlink{"/dev/stdin", "/proc/self/fd/0"},
	&Symlink{"/dev/stdout", "/proc/self/fd/1"},
	&Symlink{"/dev/stderr", "/proc/self/fd/2"},

	// mount tmpfs on /run and /tmp (which should already exist)
	&Mount{"tmpfs", "/run", "tmpfs", syscall.MS_NODEV | syscall.MS_NOSUID | syscall.MS_NOEXEC, "mode=0755"},
	&Mount{"tmpfs", "/tmp", "tmpfs", syscall.MS_NODEV | syscall.MS_NOSUID | syscall.MS_NOEXEC, ""},

	// mount shm and devpts
	&Mkdir{"/dev/shm", 0755},
	&Mount{"shm", "/dev/shm", "tmpfs", syscall.MS_NODEV | syscall.MS_NOSUID | syscall.MS_NOEXEC, ""},
	&Mkdir{"/dev/pts", 0755},
	&Mount{"devpts", "/dev/pts", "devpts", syscall.MS_NOSUID | syscall.MS_NOEXEC, ""},

	// mount /sys (which should already exist)
	&Mount{"sysfs", "/sys", "sysfs", syscall.MS_NODEV | syscall.MS_NOSUID | syscall.MS_NOEXEC, ""},
	&Mount{"cgroup_root", "/sys/fs/cgroup", "tmpfs", syscall.MS_NODEV | syscall.MS_NOSUID | syscall.MS_NOEXEC, "mode=0755"},
}

func initDev() error {
	err := syscall.Mount("dev", "/dev", "devtmpfs", syscall.MS_NOSUID|syscall.MS_NOEXEC, "")
	// /dev will be already mounted if devtmpfs.mount = 1 on the kernel
	// command line or CONFIG_DEVTMPFS_MOUNT is set. Do not consider this
	// an error.
	if err != nil && !errors.Is(err, syscall.EBUSY) {
		return err
	}
	return nil
}

func warn(str string) {
	_, _ = os.Stderr.WriteString(str + "\n")
}

func die(str string, err error) {
	warn(str + ": " + err.Error())
	errNo := new(syscall.Errno)
	if errors.As(err, errNo) {
		os.Exit(int(*errNo))
	}
	os.Exit(1)
}

func (m *Mount) doOrDie() {
	err := syscall.Mount(m.source, m.target, m.mType, m.flags, m.data)
	if err != nil {
		die("mount: "+m.target, err)
	}
}

func (m *Mkdir) doOrDie() {
	err := syscall.Mkdir(m.path, m.mode)
	if err != nil {
		warn("mkdir: " + m.path)
		if !errors.Is(err, syscall.EEXIST) {
			die("", err)
		}
	}
}
func (s *Symlink) doOrDie() {
	err := syscall.Symlink(s.target, s.linkpath)
	if err != nil {
		warn("symlink: " + s.linkpath)
		if errors.Is(err, syscall.EEXIST) {
			die("", err)
		}
	}
}

func initFsOrDie(ops []InitOp) {
	for _, op := range ops {
		op.doOrDie()
	}
}

func initCgroups() error {
	cgroupsPath := "/proc/cgroups"
	f, err := os.Open(cgroupsPath)
	if err != nil {
		return newErr(err, "fopen"+cgroupsPath)
	}
	defer func(f *os.File) {
		_ = f.Close()
	}(f)

	r := bufio.NewReader(f)

	// read and discard first line
	_, err = r.ReadString('\n')

	if err != nil {
		return newErr(err, "readstring: failed to skip first line")
	}

	basePath := "/sys/fs/cgroup/"

	for {
		var (
			name                  string
			hier, groups, enabled int
		)
		r, err := fmt.Fscanf(r, "%64s %d %d %d\n", &name, &hier, &groups, &enabled)
		if errors.Is(err, io.EOF) {
			break
		}
		if err != nil {
			return newErr(err, "fscanf: unexpected error while reading cgroups")
		}
		if r != 4 {
			return newErr(err, fmt.Sprint("fscanf: got unexpected line, expected 4 elements and got ", r))
		}
		if enabled != 0 {
			path := basePath + name
			if err := syscall.Mkdir(path, uint32(0755)); err != nil {
				return newErr(err, "mkdir: "+path)
			}
			mountFlags := uintptr(syscall.MS_NODEV | syscall.MS_NOSUID | syscall.MS_NOEXEC)
			if err := syscall.Mount(name, path, "cgroup", mountFlags, name); err != nil {
				return newErr(err, "mount: "+path)
			}
		}
	}
	return nil
}

func initConsole() error {
	conslePath := "/dev/console"
	var err error
	_ = os.Stdin.Close()
	os.Stdin, err = os.OpenFile(conslePath, os.O_RDONLY|os.O_CREATE, 0666)
	if err != nil {
		return newErr(err, "OpenFile failed for stdin")
	}
	_ = os.Stdout.Close()
	os.Stdout, err = os.OpenFile(conslePath, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0666)
	if err != nil {
		return newErr(err, "OpenFile failed for stdout")
	}
	warn(fmt.Sprint("opened stdout with fd=", os.Stdout.Fd()))
	_ = os.Stderr.Close()
	os.Stderr, err = os.OpenFile(conslePath, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0666)
	if err != nil {
		return newErr(err, "OpenFile failed for stderr")
	}
	warn(fmt.Sprint("opened stderr with fd=", os.Stdout.Fd()))
	return nil
}

// launch differs from the original C implementation because Go runtimes cannot
// fork(), so we cannot easily setsid() and setpgid() via the libc API
func launch(cmd string, args []string, env []string) (pd int, err error) {
	if env == nil || len(env) == 0 {
		env = []string{DEFAULT_PATH_ENV}
	}
	attrs := &syscall.ProcAttr{
		Env:   env,
		Files: []uintptr{os.Stdin.Fd(), os.Stdout.Fd(), os.Stderr.Fd()},
		Sys: &syscall.SysProcAttr{
			Setsid: true,
			//Setpgid: true,
		},
	}
	pid, err := syscall.ForkExec(cmd, args, attrs)
	if err != nil {
		return 0, newErr(err, "failed to fork init process")
	}
	return pid, nil
}

// consider using https://github.com/hashicorp/go-reap
// which is perfectly suited for this
func reapUntil(pid int) syscall.Signal {
	for {
		status := new(syscall.WaitStatus)
		wpid, err := syscall.Wait4(-1, status, syscall.WNOHANG, nil)
		switch err {
		case nil:
			// the child we were waiting for died, return and pass the exit status
			if wpid == pid {
				if status.Exited() {
					if status.Signal() != 0 {
						warn("child exited with error")
					}
					return status.Signal()
				}
				warn("child exited by signal")
				return 128 + status.Signal()
			}

		case syscall.ECHILD:
			// no more children
			return 0
		default:
			// an error we did not expect
			return 128
		}
	}
}

func readFile(path string) ([]string, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, newErr(err, "failed to open "+path)
	}

	defer func() {
		if f != nil {
			_ = f.Close()
		}
	}()

	var contents []string
	s := bufio.NewScanner(f)
	for s.Scan() {
		contents = append(contents, s.Text())
	}
	return contents, nil
}

func enclaveReady() error {
	socket, err := unix.Socket(unix.AF_VSOCK, unix.SOCK_STREAM, 0)
	if err != nil {
		return newErr(err, "failed open socket")
	}
	err = unix.Connect(socket, &unix.SockaddrVM{
		CID:   VSOCK_CID,
		Port:  VSOCK_PORT,
		Flags: 0,
	})
	if err != nil {
		return newErr(err, "failed to connect")
	}
	_, err = unix.Write(socket, []byte{HEART_BEAT})
	if err != nil {
		return newErr(err, "failed to write heartbeat")
	}

	buf := make([]byte, 1)
	_, err = unix.Read(socket, buf)
	if err != nil {
		return newErr(err, "failed to read heartbeat")
	}
	if buf[0] != HEART_BEAT {
		return errors.New("received wrong heartbeat")
	}

	err = unix.Close(socket)
	if err != nil {
		return newErr(err, "close vsock")
	}
	return nil
}

func initNsm() {
	if _, err := os.Stat(NSM_PATH); errors.Is(err, os.ErrNotExist) {
		warn(NSM_PATH + " does not exist - expecting linux kernel 6.8+")
		return
	}

	f, err := os.OpenFile(NSM_PATH, syscall.O_RDONLY|syscall.O_CLOEXEC, 0666)
	if err != nil {
		panic(err.Error())
	}
	var _p0 *byte
	_p0, err = unix.BytePtrFromString("")
	if err != nil {
		panic(err.Error())
	}
	rc, _, err := unix.Syscall(unix.SYS_FINIT_MODULE, f.Fd(), uintptr(unsafe.Pointer(_p0)), 0)
	if rc < 0 {
		panic("failed to insert nsm driver")
	}

	_ = f.Close()
}

// based on https://github.com/aws/aws-nitro-enclaves-sdk-bootstrap/blob/main/init/init.c
func main() {
	if err := initDev(); err != nil {
		die("failed to init dev", err)
	}
	if err := initConsole(); err != nil {
		die("failed to init console", err)
	}
	initNsm()
	if err := enclaveReady(); err != nil {
		die("failed to notify readiness", err)
	}

	env, err := readFile("/env")
	if err != nil {
		die("failed to read /env", err)
	}
	cmd, err := readFile("/cmd")
	if err != nil {
		die("failed to read /cmd", err)
	}

	if err := unix.Chdir("/rootfs"); err != nil {
		die("failed to chdir", err)
	}
	if err := unix.Chroot("/rootfs"); err != nil {
		die("failed to chroot", err)
	}

	if err := initDev(); err != nil {
		die("failed to init dev", err)
	}

	initFsOrDie(ops)

	if err := initCgroups(); err != nil {
		die("failed to init cgroups", err)
	}

	pid, err := launch(cmd[0], cmd, env)

	if err != nil {
		die("failed to launch", err)
	}

	warn("launched cmd=" + cmd[0])

	signal := reapUntil(pid)

	warn("reaped all children, returned with signal=" + signal.String())

	_ = unix.Reboot(RB_AUTOBOOT)
}
