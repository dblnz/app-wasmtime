;; hello.wat — Core WebAssembly module
;;
;; Imports `env.print_i32` from the host and calls it with 42 from `_start`.
;; Compile: wasmtime compile --target x86_64-unikraft-unknown-unknown -C cranelift-baseline -W gc=n hello.wat -o hello.cwasm
(module
  (import "env" "print_i32" (func $print_i32 (param i32)))

  (func (export "_start")
    ;; Print 42 via host function
    i32.const 42
    call $print_i32

    ;; Print 7 to show multiple calls work
    i32.const 7
    call $print_i32
  )
)
