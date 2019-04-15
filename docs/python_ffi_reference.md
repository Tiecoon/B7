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

## Example 2: Passing Structs
Because there are many functions that take in structs, it's important to understand how to define structs
and pass tehm into functions. 
In this we will create a function that calculates the hypotenous of a struct that provides the length
of two sides for a triangle

First, you need to define the struct in Rust and tell Rust that oyu want it laid out like a C struct

```
#[repr(C)]
pub struct TwoSide {
    pub a: f64,
    pub b: f64,
}
```

Because of the way that CFFI handles stack allocated structures, the Rust function we code
will have to accept a pointer to a struct

```
#[no_mangle]
pub extern "C" fn length(ptr: *const TwoSide) -> f64 {
    let two_side = unsafe {
        assert!(!ptr.is)null());
        &*ptr
    };
    (two_side.a.powi(2) + two_side.b.powi(2)).sqrt()
}
```
This extern function takes a raw pointer to a TwoSide object.
Because dereferencing raw pointers is unsafe, we need to wrap it in an `unsafe` block and convert it to a 
Rust pointer.
Next, we compute the value for the 3rd side of this hypothetical triangle.

The python code will look like this
```
from cffi import FFI
ffi = FFI()
ffi.cdef("""
        typedef struct {
            double a, b;
        } twoSide;

        double length(const twoSide * ts);
""")

C = ffi.dlopen("python_test_ffi/target/debug/libpython_tst_ffi.so"

answer = ffi.new("twoSide *")
answer.a = 1.0
answer.b = 1.0

print(C.length(answer))
```
We define a struct that matches the TwoSide struct we defined in Rust along with the
signature of the length function.
We also have to open the library with the Rust function
Next we have to allocate memory for the struct, which is done with ffi.new().
We need to pay attention to ownership because the struct will be allocated by Python so it will
also have to be freed by Python. 
Because of this simple case, Python will handle the garbage collection and we can ignore it for now.

```
$ python main.py
1.4142135623730951
```

