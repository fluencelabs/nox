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

use crate::BuiltinServicesApi;

use host_closure::{Args, Closure};
use ivalue_utils::{as_record, IType, IValue};
use particle_actors::HostImportDescriptor;

use std::sync::Arc;

type ClosureDescriptor = Arc<dyn Fn() -> HostImportDescriptor + Send + Sync + 'static>;

#[derive(Clone)]
pub struct HostClosures {
    pub create_service: Closure,
    pub call_service: Closure,
    pub builtin: Closure,
    pub add_module: Closure,
    pub add_blueprint: Closure,
}

impl HostClosures {
    pub fn descriptor(self) -> ClosureDescriptor {
        Arc::new(move || {
            let this = self.clone();
            HostImportDescriptor {
                host_exported_func: Box::new(move |_, args| this.route(args)),
                argument_types: vec![IType::String, IType::String, IType::String],
                output_type: Some(IType::Record(0)),
                error_handler: None,
            }
        })
    }

    fn route(&self, args: Vec<IValue>) -> Option<IValue> {
        let args = match Args::parse(args) {
            Ok(args) => args,
            Err(err) => {
                log::warn!("error parsing args: {:?}", err);
                return as_record(Err(IValue::String(err.to_string())));
            }
        };
        log::info!("Router args: {:?}", args);
        // route
        match args.service_id.as_str() {
            "create" => (self.create_service)(args),
            "add_module" => (self.add_module)(args),
            "add_blueprint" => (self.add_blueprint)(args),
            s if BuiltinServicesApi::is_builtin(&s) => (self.builtin)(args),
            _ => (self.call_service)(args),
        }
    }
}
