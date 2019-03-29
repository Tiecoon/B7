use crate::bindings::*;
use crate::errors::*;
use crate::brute::*;
use crate::generators::Input;
use crate::process::Process;
use libc::{c_int, c_void, ioctl, pid_t, syscall};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::FromRawFd;
use std::time::Duration;

// syscall number for perf syscall
const PERF_EVENT_OPEN_SYSCALL: i64 = 298;

// initiliaze perf on a process
fn perf_event_open(
    hw_event: *const perf_event_attr,
    pid: pid_t,
    cpu: i32,
    group_fd: i32,
    flags: u64,
) -> i32 {
    unsafe { syscall(PERF_EVENT_OPEN_SYSCALL, hw_event, pid, cpu, group_fd, flags) as i32 }
}

// perform struct setup and clear the perf file descriptor
fn get_perf_fd(pid: pid_t) -> Result<i32, SolverError> {
    let mut pe: perf_event_attr = unsafe { mem::zeroed() };

    // perf struct setup
    pe.type_ = perf_type_id_PERF_TYPE_HARDWARE;
    pe.size = mem::size_of::<perf_event_attr>() as u32;
    pe.config = u64::from(perf_hw_id_PERF_COUNT_HW_INSTRUCTIONS);
    pe.set_disabled(1);
    pe.set_exclude_kernel(1);
    pe.set_exclude_hv(1);
    pe.set_exclude_idle(1);
    pe.set_exclude_callchain_kernel(1);

    let fd = perf_event_open(&pe as *const perf_event_attr, pid, -1, -1, 0);
    if fd == -1 {
        return Err(SolverError::new(Runner::IoError, "perf_event_open failed!"));
    }

    // reset perf to make sure it is zero
    unsafe {
        ioctl(fd, 9219, 0); // PERF_EVENT_IOC_RESET
        ioctl(fd, 9216, 0); // PERF_EVENT_IOC_ENABLE
    }
    Ok(fd)
}

// read the instruction count stoed if perf is establised
fn perf_get_inst_count(fd: c_int) -> Result<i64, SolverError> {
    let mut count: i64 = 0;
    match unsafe { libc::read(fd, &mut count as *mut i64 as *mut c_void, 8) as i64 } {
        8 => Ok(count),
        x if x >= 0 => Err(SolverError::new(
            Runner::IoError,
            &format!("Perf only read {} bytes!", x),
        )),
        _ => Err(SolverError::new(
            Runner::IoError,
            "Could not read from perf fd",
        )), //TODO implement from Nix error
    }
}

#[derive(Copy, Clone)]
pub struct PerfSolver;

impl InstCounter for PerfSolver {
    // Handles basic proc spawning and running under perf
    fn get_inst_count(&self, data: &InstCountData) -> Result<i64, SolverError> {
        // TODO: error checking...
        let mut process = Process::new(&data.path);
        for arg in data.inp.argv.iter() {
            process.arg(OsStr::from_bytes(arg));
        }
        process.input(data.inp.stdin.clone());
        process.with_ptrace(true);


        //println!("Starting process!");
        let handle = process.spawn();
        let fd = get_perf_fd(handle.pid().as_raw())?;
        handle.finish(Duration::new(5, 0))?;

        // Process instruction count
        let ret = perf_get_inst_count(fd);
        drop(unsafe { File::from_raw_fd(fd) });

        ret

    }
}
