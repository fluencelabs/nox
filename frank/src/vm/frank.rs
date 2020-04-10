/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::vm::service::FluenceService;
use crate::{vm::config::Config, vm::errors::FrankError, vm::frank_result::FrankResult};

use failure::_core::marker::PhantomData;
use sha2::{digest::generic_array::GenericArray, digest::FixedOutput, Digest, Sha256};
use wasmer_runtime::{func, imports, instantiate, Ctx, Func, Instance};
use wasmer_runtime_core::memory::ptr::{Array, WasmPtr};

pub struct Frank {
    instance: &'static Instance,

    // It is safe to use unwrap() while calling these functions because Option is used here
    // to allow partially initialization of the struct. And all Option fields will contain
    // Some if invoking Frank::new is succeed.
    allocate: Option<Func<'static, i32, i32>>,
    deallocate: Option<Func<'static, (i32, i32), ()>>,
    invoke: Option<Func<'static, (i32, i32), i32>>,

    _tag: PhantomData<&'static Instance>,
}

impl Drop for Frank {
    // The manually drop is needed because at first we need to delete functions
    // and only then instance.
    fn drop(&mut self) {
        #[allow(clippy::drop_copy)]
        drop(self.allocate.as_ref());

        #[allow(clippy::drop_copy)]
        drop(self.deallocate.as_ref());

        #[allow(clippy::drop_copy)]
        drop(self.invoke.as_ref());
    }
}

impl Frank {
    /// Writes given value on the given address.
    fn write_to_mem(&mut self, address: usize, value: &[u8]) -> Result<(), FrankError> {
        let memory = self.instance.context().memory(0);

        for (byte_id, cell) in memory.view::<u8>()[address as usize..(address + value.len())]
            .iter()
            .enumerate()
        {
            cell.set(value[byte_id]);
        }

        Ok(())
    }

    /// Reads given count of bytes from given address.
    fn read_result_from_mem(&self, address: usize) -> Result<Vec<u8>, FrankError> {
        let memory = self.instance.context().memory(0);

        let mut result_size: usize = 0;

        for (byte_id, cell) in memory.view::<u8>()[address..address + 4].iter().enumerate() {
            result_size |= (cell.get() as usize) << (8 * byte_id);
        }

        let mut result = Vec::<u8>::with_capacity(result_size);
        for cell in memory.view()[(address + 4) as usize..(address + result_size + 4)].iter() {
            result.push(cell.get());
        }

        Ok(result)
    }

    /// Creates a new virtual machine executor.
    pub fn new(module: &[u8], config: Config) -> Result<Self, FrankError> {
        let import_objects = imports! {
            // this will enforce Wasmer to register EnvModule in the ctx.data field
            "logger" => {
                "log_utf8_string" => func!(logger_log_utf8_string),
            },
        };

        let instance: &'static mut Instance =
            Box::leak(Box::new(instantiate(module, &import_objects)?));

        Ok(Self {
            instance,
            allocate: Some(instance.exports.get(&config.allocate_function_name)?),
            deallocate: Some(instance.exports.get(&config.deallocate_function_name)?),
            invoke: Some(instance.exports.get(&config.invoke_function_name)?),
            _tag: PhantomData,
        })
    }
}

impl FluenceService for Frank {
    /// Invokes a main module supplying byte array and expecting byte array with some outcome back.
    fn invoke(&mut self, fn_argument: &[u8]) -> Result<FrankResult, FrankError> {
        // allocate memory for the given argument and write it to memory
        let argument_len = fn_argument.len() as i32;
        let argument_address = if argument_len != 0 {
            let address = self.allocate.as_ref().unwrap().call(argument_len)?;
            self.write_to_mem(address as usize, fn_argument)?;
            address
        } else {
            0
        };

        // invoke a main module, read a result and deallocate it
        let result_address = self
            .invoke
            .as_ref()
            .unwrap()
            .call(argument_address, argument_len)?;
        let result = self.read_result_from_mem(result_address as _)?;

        self.deallocate
            .as_ref()
            .unwrap()
            .call(result_address, result.len() as i32)?;

        Ok(FrankResult::new(result))
    }

    /// Computes the virtual machine state.
    fn compute_state_hash(&mut self) -> GenericArray<u8, <Sha256 as FixedOutput>::OutputSize> {
        let mut hasher = Sha256::new();
        let memory = self.instance.context().memory(0);

        let wasm_ptr = WasmPtr::<u8, Array>::new(0 as _);
        let raw_mem = wasm_ptr
            .deref(memory, 0, (memory.size().bytes().0 - 1) as _)
            .expect("frank: internal error in compute_vm_state_hash");
        let raw_mem: &[u8] = unsafe { &*(raw_mem as *const [std::cell::Cell<u8>] as *const [u8]) };

        hasher.input(raw_mem);
        hasher.result()
    }
}

// Prints utf8 string of the given size from the given offset.
fn logger_log_utf8_string(ctx: &mut Ctx, offset: i32, size: i32) {
    let wasm_ptr = WasmPtr::<u8, Array>::new(offset as _);
    match wasm_ptr.get_utf8_string(ctx.memory(0), size as _) {
        Some(msg) => print!("{}", msg),
        None => print!("frank logger: incorrect UTF8 string's been supplied to logger"),
    }
}
