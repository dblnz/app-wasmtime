# Test Wasm Modules

## Source Files

| File        | Type        | Description                                     |
| ----------- | ----------- | ----------------------------------------------- |
| `hello.wat` | Core module | Imports `env.print_i32`, calls it with 42 and 7 |
| `add.wat`   | Core module | Exports `add(i32, i32) -> i32`                  |

## Compiling to `.cwasm`

You **must** use wasmtime CLI v45.0.0 to match the library version:

```bash
# Install wasmtime CLI v45.0.0
curl https://wasmtime.dev/install.sh -sSf | bash -s -- --version v45.0.0

# Or with cargo:
cargo install wasmtime-cli --version 45.0.0

# Compile the modules (MUST match the Unikraft target triple and runtime config)
# All non-essential wasm features are disabled to match the no_std engine config.
cd app-ukwasmtime/wasm
WASM_FLAGS="--target x86_64-unikraft-unknown-unknown -C cranelift-baseline -W gc=n -W gc-support=n -W concurrency-support=n -W threads=n -W shared-memory=n -W relaxed-simd=n -W simd=n -W reference-types=n -W function-references=n -W tail-call=n -W multi-memory=n -W memory64=n -W exceptions=n -W custom-page-sizes=n -W wide-arithmetic=n -W extended-const=n -W component-model=n"
wasmtime compile $WASM_FLAGS hello.wat -o hello.cwasm
wasmtime compile $WASM_FLAGS add.wat -o add.cwasm
```

The resulting `.cwasm` files are precompiled for x86_64 and will be loaded
via `Module::deserialize()` at runtime inside the unikernel.

## Packaging into initrd

When running with `kraft run`, pass the `.cwasm` files as initrd:

```bash
kraft run --rm --plat qemu --arch x86_64 -M 1024 \
  --initrd wasm/hello.cwasm:wasm/add.cwasm
```

They will appear at `/hello.cwasm` and `/add.cwasm` in the unikernel filesystem.
