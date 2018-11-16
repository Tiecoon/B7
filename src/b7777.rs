use bindings::*;
use libc::{ioctl, pid_t, syscall};
use std::mem;
use std::process::exit;

const PERF_EVENT_OPEN_SYSCALL: i64 = 298;

fn perf_event_open_rs(
    hw_event: *const perf_event_attr,
    pid: pid_t,
    cpu: i32,
    group_fd: i32,
    flags: u64,
) -> i32 {
    unsafe { syscall(PERF_EVENT_OPEN_SYSCALL, hw_event, pid, cpu, group_fd, flags) as i32 }
}

pub fn get_perf_fd(pid: pid_t) -> i32 {
    let mut pe: perf_event_attr = unsafe { mem::zeroed() };

    pe.type_ = perf_type_id_PERF_TYPE_HARDWARE;
    pe.size = mem::size_of::<perf_event_attr>() as u32;
    pe.config = perf_hw_id_PERF_COUNT_HW_INSTRUCTIONS as u64;
    pe.set_disabled(1);
    pe.set_exclude_kernel(1);
    pe.set_exclude_hv(1);
    pe.set_exclude_idle(1);
    pe.set_exclude_callchain_kernel(1);

    let fd = perf_event_open_rs(&pe as *const perf_event_attr, pid, -1, -1, 0);
    if fd == -1 {
        error!("perf_event_open failed!");
        exit(-1);
    }
    unsafe {
        ioctl(fd, 9219, 0); // PERF_EVENT_IOC_RESET
        ioctl(fd, 9216, 0); // PERF_EVENT_IOC_ENABLE
    }
    fd
}