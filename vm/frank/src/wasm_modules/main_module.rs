/*
use wasmer_runtime::{error, Func, Instance, Ctx};

struct MainModule<'a> {
    invoke_func: Func<'a, (i32, i32), (i32)>,
    allocate_func: Func<'a, (i32), (i32)>,
    deallocate_func: Func<'a, (i32, i32), ()>,
}

impl MainModule {
    pub fn new(instance: Instance) -> error::Result<Self> {
        let invoke_func = instance.func("invoke")?;
        let allocate_func = instance.func("allocate")?;
        let deallocate_func = instance.func("deallocate")?;

        Ok(Self {
            invoke_func,
            allocate_func,
            deallocate_func
        })
    }

    // writes given value on the given address
    fn write_to_mem(&mut self, address: usize, value: &[u8]) -> error::Result<()> {
/*        let memory = ctx.memory(address as u32);

        let mut byte_id = 0;
        for cell in memory.view()[0 as usize .. value.len()].iter() {
            cell.set(value[byte_id]);
            byte_id += 1;
        }
*/
        Ok(())
    }

    // reads given count of bytes from given address
    fn read_result_from_mem(&self, address: usize) -> error::Result<Vec<u8>> {
/*        let memory = ctx.memory(address as u32);

        let mut result_size: usize = 0;

        let mut byte_id = 0;
        for cell in memory.view::<u8>()[0 .. 4].iter() {
            result_size |= (cell.get() << 8*byte_id) as usize;
            byte_id += 1;
        }

        let mut result = Vec::<u8>::with_capacity(result_size);

        for cell in memory.view()[4 as usize .. result_size].iter() {
            result.push(cell.get());
        }
*/
        Ok(Vec::<u8>::new())
    }

    pub fn invoke(&mut self, fn_argument: &[u8]) -> error::Result<Vec<u8>> {
        let allocated_address = self.allocate_func.call(fn_argument.len() as i32)?;
        self.write_to_mem(allocated_address as usize, fn_argument)?;

        let result_address = self
            .invoke_func
            .call(allocated_address, fn_argument.len() as i32)?;

        let result = self.read_result_from_mem(result_address as usize)?;

        self.deallocate_func.call(result_address, fn_argument.len() as i32)?;

        Ok(result)
    }
}
*/
