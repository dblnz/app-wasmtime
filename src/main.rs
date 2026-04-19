use wasmtime::component::{Component, HasSelf, Linker, Val};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

mod bindings {
    wasmtime::component::bindgen!({
        world: "test",
        path: "wit/test.wit"
    });
}

struct MyState;

struct HostState {
    wasi: WasiCtx,
    table: ResourceTable,
    my_host: MyState,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl bindings::host::Host for MyState {
    fn multiply(&mut self, a: f32, b: f32) -> f32 {
        a * b
    }
}

// const COMPONENT: [u8; usize] = include!("/src/comp_example.wasm");
static COMPONENT: [u8; include_bytes!("/src/comp_example.aot").len()] =
    *include_bytes!("/src/comp_example.aot");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_stdout()
        .inherit_env()
        .inherit_stderr()
        .build();

    let state = HostState {
        wasi,
        table: ResourceTable::new(),
        my_host: MyState,
    };
    let mut config = Config::new();
    config.wasm_component_model(true);
    //config.with_custom_code_memory(Some(alloc::sync::Arc::new(platform::WasmtimeCodeMemory {})));
    let engine = Engine::new(&config).unwrap();
    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    bindings::host::add_to_linker::<_, HasSelf<_>>(&mut linker, |state: &mut HostState| {
        &mut state.my_host
    })?;

    let component = unsafe { Component::deserialize(&engine, &COMPONENT)? };

    let mut store = Store::new(&engine, state);
    let instance = linker.instantiate(&mut store, &component)?;

    let func = instance
        .get_func(&mut store, "convert-celsius-to-fahrenheit")
        .expect("convert-celsius-to-fahrenheit does not exist");

    // let test = Test::instantiate(&mut store, &component, &linker)?;
    // let result = test.call_convert_celsius_to_fahrenheit(&mut store, 23.4)?;

    let w_params = vec![Val::Float32(23.4)];
    let mut results = vec![Val::Float32(0f32)];

    let _res = func.call(&mut store, &w_params, &mut results)?;
    if let Val::Float32(r) = results[0] {
        println!("{}", r);
    }

    Ok(())
}
