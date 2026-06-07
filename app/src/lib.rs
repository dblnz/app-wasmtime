#![no_std]
#![no_main]

pub use ukrust;

use alloc::vec;
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    safe fn printf(fmt: *const c_char, ...) -> c_int;
    safe fn open(path: *const c_char, flags: c_int) -> c_int;
    safe fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    safe fn close(fd: c_int) -> c_int;
    safe fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
}

const O_RDONLY: c_int = 0;
const SEEK_END: c_int = 2;
const SEEK_SET: c_int = 0;

fn print(msg: &[u8]) {
    printf(b"%s\0".as_ptr() as *const c_char, msg.as_ptr());
}

fn read_file(path: &[u8]) -> Option<Vec<u8>> {
    let fd = open(path.as_ptr() as *const c_char, O_RDONLY);
    if fd < 0 {
        printf(
            b"ERROR: cannot open %s\n\0".as_ptr() as *const c_char,
            path.as_ptr(),
        );
        return None;
    }

    let size = lseek(fd, 0, SEEK_END);
    if size < 0 {
        printf(b"ERROR: lseek failed\n\0".as_ptr() as *const c_char);
        close(fd);
        return None;
    }
    lseek(fd, 0, SEEK_SET);

    let size = size as usize;
    let mut buf = vec![0u8; size];
    let mut total = 0usize;
    while total < size {
        let n = read(
            fd,
            unsafe { buf.as_mut_ptr().add(total) } as *mut c_void,
            size - total,
        );
        if n <= 0 {
            break;
        }
        total += n as usize;
    }
    close(fd);

    if total != size {
        printf(
            b"ERROR: read %zu of %zu bytes\n\0".as_ptr() as *const c_char,
            total,
            size,
        );
        return None;
    }

    Some(buf)
}

fn demo_hello_module() {
    print(b"=== Running hello module ===\n\0");

    let bytes = match read_file(b"/hello.cwasm\0") {
        Some(b) => {
            printf(b"  read %zu bytes\n\0".as_ptr() as *const c_char, b.len());
            b
        },
        None => return,
    };

    match lib_ukwasmtime::run_module(&bytes) {
        Ok(()) => print(b"hello module finished OK\n\0"),
        Err(_) => print(b"ERROR: hello module failed\n\0"),
    }
}

fn demo_add_module() {
    print(b"=== Running add module ===\n\0");

    let bytes = match read_file(b"/add.cwasm\0") {
        Some(b) => b,
        None => return,
    };

    let (a, b) = (3, 4);
    match lib_ukwasmtime::call_module_ii_i(&bytes, "add", a, b) {
        Ok(result) => {
            printf(
                b"add(%d, %d) = %d\n\0".as_ptr() as *const c_char,
                a,
                b,
                result,
            );
        },
        Err(_) => print(b"ERROR: add call failed\n\0"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn uk_wasmtime_main() -> c_int {
    print(b"app-ukwasmtime: Unikraft WebAssembly demo\n\0");

    demo_hello_module();
    demo_add_module();

    print(b"app-ukwasmtime: done\n\0");
    0
}
