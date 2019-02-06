mod bindings;

use bindings::*;
use std::os::raw::{c_char, c_void, c_int};
use std::ffi::{CString, CStr};

#[allow(non_upper_case_globals)]
#[no_mangle]
pub static _USES_DR_VERSION_: c_int = bindings::_USES_DR_VERSION_;

#[allow(non_upper_case_globals)]
#[no_mangle]
pub fn dr_client_main(id: client_id_t, argc: c_int, argv: *const *const c_char) {
    println!("Hello from dr client: {}", id);
}


fn main() {
    //println!("Hello, world!");
    unsafe {
        let bin_path = CString::new(std::env::current_exe().unwrap().to_str().unwrap()).unwrap();
        //let bin_path = CString::new("/home/aaron/repos/B7/dynamorio-sys/dynamorio/build/api/bin/libinscount.so").unwrap();

        let app_name = CString::new("/home/aaron/repos/B7/dynamorio-sys/test_bin").unwrap();
        //let app_name = CString::new("/home/aaron/repos/B7/tests/wyvern".to_string()).unwrap();
        let app_name_ptr: *const c_char = app_name.as_ptr();
        let args = vec![CString::new("argone").unwrap(), CString::new("argtwo").unwrap()];

        let mut arg_ptrs: Vec<*const c_char> = args.iter().map(|s| s.as_ptr()).collect();
        let raw_arg_ptr: *mut *const c_char = arg_ptrs.as_mut_ptr();

        let mut inject_data: *mut c_void = std::ptr::null_mut();
        let inject_data_ptr: *mut *mut c_void = &mut inject_data as *mut *mut c_void;

        dr_app_setup();
        let res = dr_inject_process_create(app_name_ptr, raw_arg_ptr, inject_data_ptr);
        let pid = dr_inject_get_process_id(inject_data);
        let raw_process = dr_inject_get_image_name(inject_data);
        let process = CStr::from_ptr(raw_process);

        let mut opts_str = CString::new("").unwrap();
        let dr_path = CString::new(".").unwrap();

        let reg_result = dr_register_process(raw_process, pid, false as bool_, /* local config */
                            dr_path.as_ptr(), dr_operation_mode_t_DR_MODE_CODE_MANIPULATION, false as bool_, /* debug */
                            dr_platform_t_DR_PLATFORM_DEFAULT, opts_str.as_ptr());

        println!("About to register client...");

        let client_result = dr_register_client(raw_process, pid, false as bool_,
                                                dr_platform_t_DR_PLATFORM_DEFAULT,
                                                42 /* client id */,
                                                0 /* priority */,
                                                bin_path.as_ptr(),
                                                opts_str.as_ptr());


        println!("Registering client: {:?} {:?}", bin_path, client_result);

        println!("Enable ptrace: {}", dr_inject_prepare_to_ptrace(inject_data));

        let inject_res = dr_inject_process_inject(inject_data, false as bool_, std::ptr::null_mut());
        println!("Inject result: {}", inject_res);
        println!("Res: {} Registration: {} Pid: {} Process: {:?}", res, reg_result, pid, process);

        println!("Running: {}", dr_inject_process_run(inject_data));

        println!("Waited: {}", dr_inject_wait_for_child(inject_data, 0));
        let exit_code = dr_inject_process_exit(inject_data, false as bool_);

        println!("Exit code: {}", exit_code);
    }
}
