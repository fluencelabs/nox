use wasmer_runtime::{error, func, imports, instantiate, Ctx, DynFunc, Func, Instance, Module, Memory};
use std::fs;

struct WasmerExecutor<'a> {
    instance: Instance,
    invoke_func: Func<'a, (i32, i32), (i32)>,
    allocate_func: Func<'a, (i32), (i32)>,
    deallocate_func: Func<'a, (i32, i32), ()>,
}

impl WasmerExecutor<'_> {
/*
    pub fn new(module_path: &str) -> error::Result<Self> {
        let wasm_code = fs::read(module_path).expect("Couldn't read provided file");
        let import_objects = imports! {
            "logger" => {
                "write" => func!(logger_write),
                "flush" => func!(logger_flush),
            },
            "env" => {
                "gas" => func!(gas_counter),
                "eic" => func!(eic),
            },
        };

        let instance = instantiate(&wasm_code, &import_objects)?;
        let invoke_func = instance.func("invoke");
        let allocate_func = instance.func("allocate");
        let deallocate_func = instance.func("deallocate");

        Ok(Self {
            instance,
            invoke_func,
            allocate_func,
            deallocate_func,
        })
    }
    */
}

fn logger_write(ctx: &mut Ctx, ptr: u32, len: u32) {

}

fn logger_flush(ctx: &mut Ctx, ptr: u32, len: u32) {

}

fn gas_counter(ctx: &mut Ctx, ptr: u32, len: u32) {

}

fn eic(ctx: &mut Ctx, ptr: u32, len: u32) {

}
