use bindings::*;
use generators::Input;
use libc::{ioctl, pid_t, syscall};
use process::Process;
use std::ffi::OsStr;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::process::exit;

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
pub fn get_perf_fd(pid: pid_t) -> i32 {
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
        error!("perf_event_open failed!");
        exit(-1);
    }

    // reset perf to make sure it is zero
    unsafe {
        ioctl(fd, 9219, 0); // PERF_EVENT_IOC_RESET
        ioctl(fd, 9216, 0); // PERF_EVENT_IOC_ENABLE
    }
    fd
}

// Handles basic proc spawning and running under perf
pub fn get_inst_count(path: &str, inp: &Input) -> i64 {
    // TODO: error checking...
    let mut proc = Process::new(path);
    for arg in inp.argv.iter() {
        proc.arg(OsStr::from_bytes(arg));
    }

    // Start Process run it to completion with all arguements
    proc.start().unwrap();
    proc.write_stdin(&inp.stdin).unwrap();
    proc.close_stdin().unwrap();
    proc.init_perf().unwrap();
    proc.finish().unwrap();

    // Process instruction count
    let ret = match proc.get_inst_count() {
        Ok(x) => x,
        Err(_) => -1,
    };
    proc.close_perf();
    ret
}
