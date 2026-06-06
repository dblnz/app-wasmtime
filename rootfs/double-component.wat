;; double-component.wat — WebAssembly Component exporting a `double` function.
;;
;; Exports `double(s32) -> s32` using the component model.
;; Compile: wasmtime compile --component --target x86_64-unikraft-unknown-unknown \
;;   -C cranelift-baseline -W gc=n -W gc-support=n -W concurrency-support=n \
;;   double-component.wat -o double-component.cwasm
(component
  (core module $m
    (func (export "double") (param i32) (result i32)
      local.get 0
      i32.const 2
      i32.mul
    )
  )
  (core instance $i (instantiate $m))
  (func (export "double") (param "x" s32) (result s32)
    (canon lift (core func $i "double"))
  )
)
