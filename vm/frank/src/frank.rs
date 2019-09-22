/*
 * Copyright 2019 Fluence Labs Limited
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

use crate::config::Config;
use crate::errors::FrankError;
use crate::frank_result::FrankResult;
use crate::modules::env_module::EnvModule;
use sha2::{digest::generic_array::GenericArray, digest::FixedOutput, Digest, Sha256};
use std::ffi::c_void;
use std::fs;
use wasmer_runtime::{func, imports, instantiate, Ctx, Func, Instance};

pub struct Frank {
    instance: Instance,
    config: Box<Config>,
}

impl Frank {
    // writes given value on the given address
    fn write_to_mem(&mut self, address: usize, value: &[u8]) -> Result<(), FrankError> {
        let memory = self.instance.context_mut().memory(0);

        for (byte_id, cell) in memory.view::<u8>()[address as usize..(address + value.len())]
            .iter()
            .enumerate()
        {
            cell.set(value[byte_id]);
        }

        Ok(())
    }

    // reads given count of bytes from given address
    fn read_result_from_mem(&self, address: usize) -> Result<Vec<u8>, FrankError> {
        let memory = self.instance.context().memory(0);

        let mut result_size: usize = 0;

        for (byte_id, cell) in memory.view::<u8>()[address..address + 4].iter().enumerate() {
            result_size |= (cell.get() << (8 * byte_id as u8)) as usize;
        }

        let mut result = Vec::<u8>::with_capacity(result_size);
        for cell in memory.view()[(address + 4) as usize..(address + result_size + 4)].iter() {
            result.push(cell.get());
        }

        Ok(result)
    }

    fn call_invoke_func(&self, addr: i32, len: i32) -> Result<i32, FrankError> {
        let func: Func<(i32, i32), (i32)> =
            self.instance.func(&self.config.invoke_function_name)?;
        let result = func.call(addr, len)?;
        Ok(result)
    }

    fn call_allocate_func(&self, size: i32) -> Result<i32, FrankError> {
        let func: Func<(i32), (i32)> = self.instance.func(&self.config.allocate_function_name)?;
        let result = func.call(size)?;
        Ok(result)
    }

    fn call_deallocate_func(&self, addr: i32, size: i32) -> Result<(), FrankError> {
        let func: Func<(i32, i32), ()> =
            self.instance.func(&self.config.deallocate_function_name)?;
        func.call(addr, size).map_err(Into::into)
    }

    pub fn invoke(&mut self, fn_argument: &[u8]) -> Result<FrankResult, FrankError> {
        let env: &mut EnvModule =
            unsafe { &mut *(self.instance.context_mut().data as *mut EnvModule) };
        env.renew_state();

        let argument_len = fn_argument.len() as i32;
        let argument_address = if argument_len != 0 {
            let address = self.call_allocate_func(argument_len)?;
            self.write_to_mem(address as usize, fn_argument)?;
            address
        } else {
            0
        };

        let result_address = self.call_invoke_func(argument_address, argument_len)?;
        let result = self.read_result_from_mem(result_address as usize)?;
        self.call_deallocate_func(result_address, result.len() as i32)?;

        let state = env.get_state();
        Ok(FrankResult::new(result, state.0, state.1))
    }

    pub fn compute_vm_state_hash(
        &mut self,
    ) -> GenericArray<u8, <Sha256 as FixedOutput>::OutputSize> {
        let mut hasher = Sha256::new();
        let memory = self.instance.context_mut().memory(0);

        for cell in memory.view::<u8>()[0 as usize..memory.size().0 as usize].iter() {
            hasher.input(&[cell.get()]);
        }

        hasher.result()
    }

    pub fn new(module_path: &str, config: Box<Config>) -> Result<Self, FrankError> {
        let wasm_code = fs::read(module_path)?;

        let env_state = move || {
            // allocate EnvModule on the heap
            let env_module = EnvModule::new();
            let dtor = (|data: *mut c_void| unsafe {
                drop(Box::from_raw(data as *mut EnvModule));
            }) as fn(*mut c_void);

            // and then release corresponding Box object obtaining the raw pointer
            (Box::leak(env_module) as *mut EnvModule as *mut c_void, dtor)
        };

        let import_objects = imports! {
            // this will enforce Wasmer to register EnvModule in the ctx.data field
            env_state,
            "logger" => {
                "write" => func!(logger_write),
                "flush" => func!(logger_flush),
            },
            "env" => {
                "gas" => func!(update_gas_counter),
                "eic" => func!(update_eic),
            },
        };

        let instance = instantiate(&wasm_code, &import_objects)?;
        Ok(Self { instance, config })
    }
}

fn logger_write(_ctx: &mut Ctx, byte: i32) {
    // TODO: since Wasmer has been landed, it is possible to optimize logging
    print!("{}", byte);
}

fn logger_flush(_ctx: &mut Ctx) {
    // TODO: since Wasmer has been landed, it is possible to optimize logging
    println!();
}

fn update_gas_counter(ctx: &mut Ctx, spent_gas: i32) {
    let env: &mut EnvModule = unsafe { &mut *(ctx.data as *mut EnvModule) };
    env.gas(spent_gas);
}

fn update_eic(ctx: &mut Ctx, eic: i32) {
    let env: &mut EnvModule = unsafe { &mut *(ctx.data as *mut EnvModule) };
    env.eic(eic);
}
