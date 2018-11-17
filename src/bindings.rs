// Needed for bindgen bindings
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unknown_lints)]
#![allow(clippy)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
