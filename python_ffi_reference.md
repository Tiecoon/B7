Reference for writing python code that calls rust functions
===========================================================

This is a reference for anyone who wants to help contribute to the python ffi for B7. This tutorial is not specific for B7 but can be very hepful in understanding how ffi's work.

## Setup
to get started we will need to use the python module CFFI. you can install it with
''' 
pip install cffi
'''


## Example 1: Basic function calls
to start, you can initialize a cargo repo with the command
```
cargo init anyName
```
where anyName is the name of the new cargo directory
now go into the src directory and find main.rto start, you can initialize a cargo repo with the command
```
cargo init anyName
```
where anyName is the name of the new cargo directory

find main.rs in the src directory and define a simple function
```
#[no_mangle]
pub extern "C" fn addTwo(x: i32) -> i32 {
    x + 2
}
```
The trick to making an ffi for rust is to code a wrapper in Python that knows how to call C functions,
and a wrapper in rust that exposes C functions and translates them into rust function calls.

the attribute 'no_mangle' tells Rust not the change the name of the function so CFFI can find it later
the keywords 'extern "C"' tells Rust to emit a function call that can be called as if it were written in C

The next thing we need to do is instruct CArgo to build the code as a dynamic library (or "dylib")
In the Cargo.toml file write
```
[lib]
name = "python_ffi"
crate-type = ["dylib"]
```

after running ''' cargo run ''' it will build the dynamic library in the target/debug directory
if you're running windows you wil get a file named 'python_ffi.dll' 
if your'e running linux you will get a file named 'libpython_ffi.so'

Next we need to write Python code to load and call the library (main.py)
```
from cffi import FFI
ffi = FFI()
ffi.cdef("""
    int addTwo(int);
""")

C = ffi.dlopen("../anyName/target/debug/libpython_ffi.so")

print(C.addTwo(10))
```
First, we import the cffi module and create an FFI object.

Next, cdef is a function from cffi that takes a string containing a function signature that matches the 'addTwo'
function we defined in Rust. We will need to do this for all functions and structs we want to expose to Python. Be careful not to define any functions using keywords in C like using "double" as a function name,
this will cause parse errors in cdef.

The next function 'dlopen' opens the library we compiled with cargo. Make sure the string contains the path
to the library relative to the directory the python code is in.

Next, we call the function "addTwo" from Python

```
$ python main.py
12
```


