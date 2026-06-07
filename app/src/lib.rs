#![no_std]
#![no_main]

pub use ukrust;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_void};
use serde::{Deserialize, Serialize};

unsafe extern "C" {
    safe fn printf(fmt: *const c_char, ...) -> c_int;
    safe fn open(path: *const c_char, flags: c_int) -> c_int;
    safe fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    safe fn close(fd: c_int) -> c_int;
    safe fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;

    safe fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    safe fn bind(fd: c_int, addr: *const SockaddrIn, len: u32) -> c_int;
    safe fn listen(fd: c_int, backlog: c_int) -> c_int;
    safe fn accept(fd: c_int, addr: *mut c_void, len: *mut u32) -> c_int;
    safe fn send(fd: c_int, buf: *const c_void, len: usize, flags: c_int) -> isize;
    safe fn recv(fd: c_int, buf: *mut c_void, len: usize, flags: c_int) -> isize;
}

const O_RDONLY: c_int = 0;
const SEEK_END: c_int = 2;
const SEEK_SET: c_int = 0;

const AF_INET: c_int = 2;
const SOCK_STREAM: c_int = 1;
const INADDR_ANY: u32 = 0;

#[repr(C)]
struct SockaddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

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

/// JSON body for `POST /add`.
#[derive(Deserialize)]
struct AddRequest {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct AddResponse {
    result: i32,
}

#[derive(Serialize)]
struct MessageResponse {
    message: &'static str,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    error: &'a str,
}

/// Wrap a JSON body in a minimal HTTP/1.1 response.
fn json_response(status: &str, body: Vec<u8>) -> Vec<u8> {
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        body.len(),
    );
    let mut out = Vec::with_capacity(head.len() + body.len());
    out.extend_from_slice(head.as_bytes());
    out.extend_from_slice(&body);
    out
}

/// Serialize `value` as the JSON body of a 200 OK response.
fn json_ok<T: Serialize>(value: &T) -> Vec<u8> {
    match serde_json::to_vec(value) {
        Ok(body) => json_response("200 OK", body),
        Err(_) => json_error("500 Internal Server Error", "serialization failed"),
    }
}

/// Build a JSON `{"error": ...}` response with the given status line.
fn json_error(status: &str, message: &str) -> Vec<u8> {
    let body = serde_json::to_vec(&ErrorResponse { error: message })
        .unwrap_or_else(|_| Vec::from(&b"{\"error\":\"internal\"}"[..]));
    json_response(status, body)
}

/// Dispatch a parsed request to a JSON response. `body` is the request body
/// (used by POST routes); `add_wasm` is the cached add.cwasm module.
fn handle_request(method: &str, path: &str, body: &[u8], add_wasm: Option<&[u8]>) -> Vec<u8> {
    // Strip a query string, if any.
    let path = path.split('?').next().unwrap_or(path);

    match path {
        "/" => match method {
            "GET" => json_ok(&MessageResponse { message: "hello from app-ukwasmtime" }),
            _ => json_error("405 Method Not Allowed", "method not allowed"),
        },
        "/health" => match method {
            "GET" => json_ok(&HealthResponse { status: "ok" }),
            _ => json_error("405 Method Not Allowed", "method not allowed"),
        },
        "/add" => match method {
            "POST" => handle_add(body, add_wasm),
            _ => json_error("405 Method Not Allowed", "use POST with a JSON body"),
        },
        _ => json_error("404 Not Found", "not found"),
    }
}

/// Handle `POST /add`: decode `{"a":int,"b":int}` and run the wasm add module.
fn handle_add(body: &[u8], add_wasm: Option<&[u8]>) -> Vec<u8> {
    let req: AddRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(_) => {
            return json_error("400 Bad Request", "invalid JSON body; expected {\"a\":int,\"b\":int}");
        },
    };
    let wasm = match add_wasm {
        Some(w) => w,
        None => return json_error("503 Service Unavailable", "add module unavailable"),
    };
    match lib_ukwasmtime::call_module_ii_i(wasm, "add", req.a, req.b) {
        Ok(result) => json_ok(&AddResponse { result }),
        Err(_) => json_error("500 Internal Server Error", "add call failed"),
    }
}

/// Read a full HTTP request from `cfd` into `buf`, honoring Content-Length.
/// Returns (method, path, body_start, total_len) on success.
fn read_request(cfd: c_int, buf: &mut [u8]) -> Option<(String, String, usize, usize)> {
    let mut filled = 0usize;

    // Read until the headers are complete.
    let (method, path, body_start, content_length) = loop {
        if filled == buf.len() {
            return None;
        }
        let n = recv(
            cfd,
            unsafe { buf.as_mut_ptr().add(filled) } as *mut c_void,
            buf.len() - filled,
            0,
        );
        if n <= 0 {
            return None;
        }
        filled += n as usize;

        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut parser = httparse::Request::new(&mut headers);
        match parser.parse(&buf[..filled]) {
            Ok(httparse::Status::Complete(hlen)) => {
                let method = String::from(parser.method.unwrap_or(""));
                let path = String::from(parser.path.unwrap_or("/"));
                let content_length = parser
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("content-length"))
                    .and_then(|h| core::str::from_utf8(h.value).ok())
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                break (method, path, hlen, content_length);
            },
            Ok(httparse::Status::Partial) => continue,
            Err(_) => return None,
        }
    };

    // Read the remainder of the body, if any.
    while filled - body_start < content_length && filled < buf.len() {
        let n = recv(
            cfd,
            unsafe { buf.as_mut_ptr().add(filled) } as *mut c_void,
            buf.len() - filled,
            0,
        );
        if n <= 0 {
            break;
        }
        filled += n as usize;
    }

    Some((method, path, body_start, filled))
}

/// HTTP server backed by the `httparse` request parser (the same crate used by
/// hyper/actix) and `serde_json` for JSON I/O. This function does not return.
fn serve_http(port: u16) {
    let sfd = socket(AF_INET, SOCK_STREAM, 0);
    if sfd < 0 {
        print(b"ERROR: socket() failed\n\0");
        return;
    }

    let addr = SockaddrIn {
        sin_family: AF_INET as u16,
        sin_port: port.to_be(),
        sin_addr: INADDR_ANY,
        sin_zero: [0; 8],
    };

    if bind(sfd, &addr, core::mem::size_of::<SockaddrIn>() as u32) != 0 {
        print(b"ERROR: bind() failed\n\0");
        close(sfd);
        return;
    }

    if listen(sfd, 16) != 0 {
        print(b"ERROR: listen() failed\n\0");
        close(sfd);
        return;
    }

    // Cache the add module once so the /add route doesn't hit the FS per request.
    let add_wasm = read_file(b"/add.cwasm\0");

    printf(
        b"HTTP server listening on port %d\n\0".as_ptr() as *const c_char,
        port as c_int,
    );

    let mut buf = [0u8; 4096];
    loop {
        let cfd = accept(sfd, core::ptr::null_mut(), core::ptr::null_mut());
        if cfd < 0 {
            continue;
        }

        let response = match read_request(cfd, &mut buf) {
            Some((method, path, body_start, total)) => {
                let body = &buf[body_start..total];
                handle_request(&method, &path, body, add_wasm.as_deref())
            },
            None => json_error("400 Bad Request", "malformed request"),
        };

        let mut sent = 0usize;
        while sent < response.len() {
            let m = send(
                cfd,
                unsafe { response.as_ptr().add(sent) } as *const c_void,
                response.len() - sent,
                0,
            );
            if m <= 0 {
                break;
            }
            sent += m as usize;
        }

        close(cfd);
        print(b"  served a request\n\0");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn uk_wasmtime_main() -> c_int {
    print(b"app-ukwasmtime: Unikraft WebAssembly demo\n\0");

    demo_hello_module();
    demo_add_module();

    print(b"app-ukwasmtime: demos done, starting HTTP server\n\0");
    serve_http(8080);

    print(b"app-ukwasmtime: done\n\0");
    0
}
