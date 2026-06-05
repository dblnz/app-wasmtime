;; add.wat — Core WebAssembly module exporting an `add` function.
;;
;; This is a plain core module (not a component). It exports `add(i32, i32) -> i32`.
;; The demo app will use Module::deserialize + get_typed_func to call it.
;;
;; Compile: wasmtime compile add.wat -o add.cwasm
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add
  )
)
