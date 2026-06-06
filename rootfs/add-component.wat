;; add-component.wat — WebAssembly Component wrapping an add function.
;;
;; Exports an `add(s32, s32) -> s32` function using the component model.
;; Compile: wasmtime compile --component --target x86_64-unikraft-unknown-unknown \
;;   -C cranelift-baseline -W gc=n -W gc-support=n -W concurrency-support=n \
;;   add-component.wat -o add-component.cwasm
(component
  (core module $m
    (func (export "add") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      i32.add
    )
  )
  (core instance $i (instantiate $m))
  (func (export "add") (param "a" s32) (param "b" s32) (result s32)
    (canon lift (core func $i "add"))
  )
)
