FROM rust:1.95.0-trixie AS build

WORKDIR /src

COPY ./src /src/src
COPY ./Cargo.toml /src/Cargo.toml
COPY ./Cargo.lock /src/Cargo.lock
COPY ./wit/test.wit /src/wit/test.wit
COPY ./comp_example.aot /src/comp_example.aot

RUN cargo build

FROM scratch

COPY --from=build src/target/debug/app-wasmtime /app
COPY --from=build /lib/x86_64-linux-gnu/libc.so.6 /lib/x86_64-linux-gnu/libc.so.6
COPY --from=build /lib/x86_64-linux-gnu/libm.so.6 /lib/x86_64-linux-gnu/libm.so.6
COPY --from=build /lib/x86_64-linux-gnu/libgcc_s.so.1 /lib/x86_64-linux-gnu/libgcc_s.so.1
COPY --from=build /lib64/ld-linux-x86-64.so.2 /lib64/ld-linux-x86-64.so.2
