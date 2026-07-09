use std::io;
use std::mem;
use std::os::unix::io::FromRawFd;
use std::process;

use rustix::event::{poll, PollFd, PollFlags};
use rustix::fd::{AsFd, BorrowedFd, OwnedFd};
use rustix::fs::{openat, statx, AtFlags, Dir, OFlags, Statx, StatxFlags, CWD};
use rustix::io::{read, retry_on_intr};
use rustix::process::{
    getgid, getpid, getuid, pidfd_open, pidfd_send_signal, waitid, Gid, Pid, PidfdFlags, Signal,
    Uid, WaitId, WaitIdOptions,
};

struct PidEntry {
    pid: Pid,
    name: Vec<u8>,
    stx: Option<Statx>,
}

fn main() -> io::Result<()> {
    let my_pid = getpid();

    if !my_pid.is_init() {
        eprintln!("That does not seem very nice.");
        process::exit(1);
    }

    let my_uid = getuid();
    let my_gid = getgid();

    eprintln!(
        "playniceplease: pid={my_pid} uid={my_uid} gid={my_gid}; waiting for SIGTERM"
    );

    let sfd = wait_for_sigterm()?;

    let proc_fd = openat(
        CWD,
        "/proc",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?;
    let mut entries = scan_proc(proc_fd.as_fd())?;
    statx_all(proc_fd.as_fd(), &mut entries)?;
    drop(proc_fd);

    let (sent, confirmed) = terminate_and_confirm(&entries, my_uid, my_gid, my_pid);
    eprintln!(
        "playniceplease: sent SIGTERM to {sent} process(es), confirmed {confirmed} terminated, exiting"
    );

    drop(sfd);
    Ok(())
}

fn wait_for_sigterm() -> io::Result<OwnedFd> {
    let mut mask: libc::sigset_t = unsafe { mem::zeroed() };
    unsafe {
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGTERM);
        if libc::sigprocmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    let raw_sfd = unsafe { libc::signalfd(-1, &mask, libc::SFD_CLOEXEC) };
    if raw_sfd < 0 {
        return Err(io::Error::last_os_error());
    }
    let sfd = unsafe { OwnedFd::from_raw_fd(raw_sfd) };

    let mut ssi = [0u8; mem::size_of::<libc::signalfd_siginfo>()];
    loop {
        let n = read(sfd.as_fd(), &mut ssi)?;
        if n == mem::size_of::<libc::signalfd_siginfo>() {
            break;
        }
    }
    eprintln!("playniceplease: SIGTERM received");
    Ok(sfd)
}

fn scan_proc(proc_fd: BorrowedFd<'_>) -> io::Result<Vec<PidEntry>> {
    let mut dir = Dir::read_from(proc_fd)?;
    let mut entries: Vec<PidEntry> = Vec::new();
    while let Some(entry) = dir.next().transpose()? {
        let name = entry.file_name().to_bytes();
        if !name.is_empty() && name.iter().all(|b| b.is_ascii_digit()) {
            if let Ok(s) = std::str::from_utf8(name) {
                if let Ok(pid) = s.parse::<i32>() {
                    if let Some(pid) = Pid::from_raw(pid) {
                        entries.push(PidEntry {
                            pid,
                            name: name.to_vec(),
                            stx: None,
                        });
                    }
                }
            }
        }
    }
    eprintln!("playniceplease: scanned {} proc entries", entries.len());
    Ok(entries)
}

fn statx_all(proc_fd: BorrowedFd<'_>, entries: &mut [PidEntry]) -> io::Result<()> {
    let mask = StatxFlags::UID | StatxFlags::GID;
    for e in entries.iter_mut() {
        match statx(proc_fd.as_fd(), &e.name, AtFlags::empty(), mask) {
            Ok(stx) => e.stx = Some(stx),
            Err(err) => {
                if !matches!(
                    err,
                    rustix::io::Errno::NOENT | rustix::io::Errno::SRCH
                ) {
                    return Err(err.into());
                }
            }
        }
    }
    Ok(())
}

fn terminate_and_confirm(
    entries: &[PidEntry],
    my_uid: Uid,
    my_gid: Gid,
    my_pid: Pid,
) -> (usize, usize) {
    let wanted = (StatxFlags::UID | StatxFlags::GID).bits();
    let my_uid_raw = my_uid.as_raw();
    let my_gid_raw = my_gid.as_raw();

    let mut targets: Vec<(Pid, OwnedFd)> = Vec::new();
    for e in entries {
        if e.pid == my_pid {
            continue;
        }
        let Some(stx) = &e.stx else {
            continue;
        };
        if stx.stx_mask & wanted != wanted {
            continue;
        }
        if stx.stx_uid != my_uid_raw && stx.stx_gid != my_gid_raw {
            continue;
        }
        match pidfd_open(e.pid, PidfdFlags::empty()) {
            Ok(fd) => targets.push((e.pid, fd)),
            Err(err) => {
                if err == rustix::io::Errno::SRCH {
                    eprintln!("playniceplease: pid={}: already exited (no pidfd)", e.pid);
                } else {
                    eprintln!("playniceplease: pid={}: cannot open pidfd ({err})", e.pid);
                }
            }
        }
    }

    let sent = targets.len();

    let mut waiters: Vec<(Pid, OwnedFd)> = Vec::with_capacity(targets.len());
    for (pid, pidfd) in targets {
        match pidfd_send_signal(pidfd.as_fd(), Signal::TERM) {
            Ok(()) => waiters.push((pid, pidfd)),
            Err(err) => {
                if err == rustix::io::Errno::SRCH {
                    eprintln!("playniceplease: pid={pid}: exited before signal");
                } else {
                    eprintln!("playniceplease: pid={pid}: pidfd_send_signal failed ({err})");
                }
            }
        }
    }

    let mut confirmed = 0usize;
    while !waiters.is_empty() {
        let mut pollfds: Vec<PollFd<'_>> = waiters
            .iter()
            .map(|(_, fd)| PollFd::new(fd, PollFlags::IN))
            .collect();
        match retry_on_intr(|| poll(&mut pollfds, None)) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("playniceplease: poll failed ({err})");
                break;
            }
        }

        let ready: Vec<bool> = pollfds
            .iter()
            .map(|pfd| pfd.revents().intersects(PollFlags::IN | PollFlags::HUP))
            .collect();

        let mut i = 0;
        while i < waiters.len() {
            if ready[i] {
                let (pid, pidfd) = waiters.swap_remove(i);
                if wait_and_report(pid, pidfd.as_fd()) {
                    confirmed += 1;
                }
            } else {
                i += 1;
            }
        }
    }

    (sent, confirmed)
}

fn wait_and_report(pid: Pid, pidfd: BorrowedFd<'_>) -> bool {
    let result = retry_on_intr(|| {
        waitid(
            WaitId::PidFd(pidfd),
            WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
        )
    });

    let status = match result {
        Ok(Some(status)) => status,
        Ok(None) => {
            eprintln!("playniceplease: pid={pid}: no status available");
            return false;
        }
        Err(rustix::io::Errno::CHILD) => {
            eprintln!("playniceplease: pid={pid}: terminated (exit status unavailable)");
            return true;
        }
        Err(err) => {
            eprintln!("playniceplease: pid={pid}: waitid failed ({err})");
            return false;
        }
    };

    if status.exited() {
        if let Some(code) = status.exit_status() {
            eprintln!("playniceplease: pid={pid}: terminated (exit code {code})");
        } else {
            eprintln!("playniceplease: pid={pid}: terminated (exit code unavailable)");
        }
    } else if status.killed() {
        if let Some(sig) = status.terminating_signal() {
            eprintln!(
                "playniceplease: pid={pid}: terminated (killed by signal {sig} {})",
                signal_name(sig)
            );
        } else {
            eprintln!("playniceplease: pid={pid}: terminated (killed by signal)");
        }
    } else if status.dumped() {
        if let Some(sig) = status.terminating_signal() {
            eprintln!(
                "playniceplease: pid={pid}: terminated (killed by signal {sig} {}, core dumped)",
                signal_name(sig)
            );
        } else {
            eprintln!("playniceplease: pid={pid}: terminated (killed by signal, core dumped)");
        }
    } else {
        eprintln!("playniceplease: pid={pid}: state changed (unrecognized)");
    }
    true
}

fn signal_name(sig: i32) -> &'static str {
    match sig {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        6 => "SIGABRT",
        8 => "SIGFPE",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        _ => "",
    }
}
