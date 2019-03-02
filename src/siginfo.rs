use static_assertions::assert_eq_size;


// This is a really weird type...
/*struct better_siginfo_t {
    si_signo: libc::c_int,
    si_errno: libc::c_int,
    si_code: libc::c_int,
    si_trapno: libc::c_int,
    si_pid: libc::pid_t,
    si_uid: libc::uid_t,
    si_status: libc::c_int,
    si_utime: libc::clock_t,
    si_stime: libc::clock_t,
    si_value: libc::sigval,
    si_int: libc::c_int,
    si_ptr: *const libc::c_void,
    si_overrun: libc::c_int,
    si_timerid: libc::c_int,
    si_addr: *const libc::c_void,
    si_band: libc::c_long,
    si_fd: libc::c_int,
    si_addr_lsb: libc::c_short,
    si_lower: *const libc::c_void,
    si_upper: *const libc::c_void,
    si_pkey: libc::c_int,
    si_call_addr: *const libc::c_void,
    si_syscall: libc::c_int,
    si_arch: libc::c_uint
}*/



#[repr(C)]
#[derive(Copy, Clone)]
pub union better_siginfo_t {
    pub fields: siginfo_fields,
    pad: [libc::c_int; 32]
}


unsafe impl Send for better_siginfo_t {}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct siginfo_fields {
    pub si_signo: libc::c_int,
    pub si_errno: libc::c_int,
    pub si_code: libc::c_int,
    pub inner: siginfo_fields_inner
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union siginfo_fields_inner {
    pub kill: siginfo_kill,
    pub timer: siginfo_timer,
    pub rt: siginfo_rt,
    pub sigchld: siginfo_sigchld
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct siginfo_kill {
    pub si_pid: libc::pid_t,
    pub si_uid: libc::uid_t
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct siginfo_timer {
    si_tid: libc::c_int,
    si_overrun: libc::c_int,
    si_value: libc::sigval,
    si_sys_privat: libc::c_int
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct siginfo_rt {
    si_pid: libc::pid_t,
    si_uid: libc::uid_t,
    si_value: libc::sigval
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct siginfo_sigchld {
    si_pid: libc::pid_t,
    si_uid: libc::uid_t,
    status: libc::c_int,
    utime: libc::c_int,
    stime: libc::c_int
}


assert_eq_size!(blah; better_siginfo_t, libc::siginfo_t);
