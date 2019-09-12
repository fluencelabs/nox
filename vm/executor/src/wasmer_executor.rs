use std::fs;
use wasmer_runtime::error::CallError;
use wasmer_runtime::{error, func, imports, instantiate, Ctx, Func, Instance};

pub struct WasmerExecutor {
    instance: Instance,

}

impl WasmerExecutor {
    // writes given value on the given address
    fn write_to_mem(&mut self, address: usize, value: &[u8]) -> error::Result<()> {
        let memory = self.instance.context_mut().memory(address as u32);

        let mut byte_id = 0;
        for cell in memory.view()[0 as usize..value.len()].iter() {
            cell.set(value[byte_id]);
            byte_id += 1;
        }

        Ok(())
    }

    // reads given count of bytes from given address
    fn read_result_from_mem(&self, address: usize) -> error::Result<Vec<u8>> {
        let memory = self.instance.context().memory(address as u32);

        let mut result_size: usize = 0;

        let mut byte_id = 0;
        for cell in memory.view::<u8>()[0..4].iter() {
            result_size |= (cell.get() << 8 * byte_id) as usize;
            byte_id += 1;
        }

        let mut result = Vec::<u8>::with_capacity(result_size);

        for cell in memory.view()[4 as usize..result_size].iter() {
            result.push(cell.get());
        }
        Ok(Vec::<u8>::new())
    }

    fn call_invoke_func(&self, addr: i32, len: i32) -> error::Result<i32> {
        let func: Func<(i32, i32), (i32)> = self.instance.func("invoke")?;
        let result = func.call(addr, len)?;
        Ok(result)
    }

    fn call_allocate_func(&self, size: i32) -> error::Result<i32> {
        let func: Func<(i32), (i32)> = self.instance.func("allocate")?;
        let result = func.call(size)?;
        Ok(result)
    }

    fn call_deallocate_func(&self, addr: i32, size: i32) -> error::Result<()> {
        let func: Func<(i32, i32), ()> = self.instance.func("deallocate")?;
        func.call(addr, size).map_err(Into::into)
    }

    pub fn invoke(&mut self, fn_argument: &[u8]) -> error::Result<Vec<u8>> {
        let argument_len = fn_argument.len() as i32;
        let allocated_region_address = self.call_allocate_func(argument_len)?;

        self.write_to_mem(allocated_region_address as usize, fn_argument)?;
        let result_address = self.call_invoke_func(allocated_region_address, argument_len)?;
        let result = self.read_result_from_mem(result_address as usize)?;
        self.call_deallocate_func(result_address, result.len() as i32)?;

        Ok(result)
    }

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
        Ok(Self { instance })
    }
}

fn logger_write(ctx: &mut Ctx, byte: i32) {
    // TODO: since Wasmer has been landed, change log to more optimal
    print!("{}", byte);
}

fn logger_flush(ctx: &mut Ctx) {
    println!();
}

fn gas_counter(ctx: &mut Ctx, eic: i32) {
}

fn eic(ctx: &mut Ctx, eic: i32) {

}
