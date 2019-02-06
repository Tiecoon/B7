mod bindings;

use bindings::*;
use std::os::raw::{c_char, c_void, c_int};
use std::ffi::CString;

#[allow(non_upper_case_globals)]
#[no_mangle]
pub static _USES_DR_VERSION_: c_int = bindings::_USES_DR_VERSION_;

fn main() {
    //println!("Hello, world!");
    unsafe {
        let app_name = CString::new("blah".to_string()).unwrap();
        let app_name_ptr: *const c_char = app_name.as_ptr();
        let args = vec![CString::new("argone").unwrap(), CString::new("argtwo").unwrap()];

        let mut arg_ptrs: Vec<*const c_char> = args.iter().map(|s| s.as_ptr()).collect();
        let raw_arg_ptr: *mut *const c_char = arg_ptrs.as_mut_ptr();

        let mut data: u8 = 0;
        let mut data_ptr = &mut data as *mut u8 as *mut c_void;
        let data_ptr_ptr = &mut data_ptr as *mut *mut c_void;

        dr_app_setup();
        let res = dr_inject_process_create(app_name_ptr, raw_arg_ptr, data_ptr_ptr);
        println!("Res: {}", res);
    }
}
