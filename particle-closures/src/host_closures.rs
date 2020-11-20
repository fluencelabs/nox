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

use host_closure::{Args, Closure};
use ivalue_utils::{IType, IValue};
use particle_actors::HostImportDescriptor;

use serde_json::{json, Value as JValue};
use std::sync::Arc;

type ClosureDescriptor = Arc<dyn Fn() -> HostImportDescriptor + Send + Sync + 'static>;

#[derive(Clone)]
pub struct HostClosures {
    pub resolve: Closure,
    pub neighborhood: Closure,
    pub create_service: Closure,
    pub call_service: Closure,
    pub add_module: Closure,
    pub add_blueprint: Closure,
    pub get_modules: Closure,
    pub get_blueprints: Closure,
    pub add_provider: Closure,
    pub get_providers: Closure,
    pub get_interface: Closure,
    pub get_active_interfaces: Closure,
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
                log::warn!("host function args parse error: {:?}", err);
                return ivalue_utils::error(json!(err.to_string()));
            }
        };
        log::info!("Host function call {:?} {}", args.service_id, args.fname);
        log::debug!("Host function call, args: {:#?}", args);
        // route
        match args.service_id.as_str() {
            "resolve" => (self.resolve)(args),
            "neighborhood" => (self.neighborhood)(args),
            "create" => (self.create_service)(args),
            "add_module" => (self.add_module)(args),
            "add_blueprint" => (self.add_blueprint)(args),
            "get_available_modules" => (self.get_modules)(args),
            "get_available_blueprints" => (self.get_blueprints)(args),
            "add_provider" => (self.add_provider)(args),
            "get_providers" => (self.get_providers)(args),
            "get_interface" => (self.get_interface)(args),
            "get_active_interfaces" => (self.get_active_interfaces)(args),
            "identity" => ivalue_utils::ok(JValue::Array(args.args)),
            _ => (self.call_service)(args),
        }
    }
}
