mod bindings;

use bindings::*;
use std::os::raw::{c_char, c_void};
use std::ffi::CString;

fn main() {
    println!("Hello, world!");
    unsafe {
        let app_name = CString::new("blah".to_string()).unwrap();
        let app_name_ptr: *const c_char = app_name.as_ptr();
        let args = vec![CString::new("argone").unwrap(), CString::new("argtwo").unwrap()];

        let mut arg_ptrs: Vec<*const c_char> = args.iter().map(|s| s.as_ptr()).collect();
        let raw_arg_ptr: *mut *const c_char = arg_ptrs.as_mut_ptr();

        let mut data: u8 = 0;
        let mut data_ptr = &mut data as *mut u8 as *mut c_void;
        let data_ptr_ptr = &mut data_ptr as *mut *mut c_void;

        let res = dr_inject_process_create(app_name_ptr, raw_arg_ptr, data_ptr_ptr);
        println!("Res: {}", res);
    }
}
