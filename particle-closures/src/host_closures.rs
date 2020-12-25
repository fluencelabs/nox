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

use host_closure::{Args, Closure, ClosureDescriptor, FCEServiceClosure, ParticleParameters};
use ivalue_utils::{ok, IValue};

use serde_json::{json, Value as JValue};
use std::sync::Arc;
use JValue::Array;

#[derive(Clone)]
pub struct HostClosures {
    pub resolve: Closure,
    pub neighborhood: Closure,
    pub create_service: Closure,
    pub call_service: FCEServiceClosure,
    pub add_module: Closure,
    pub add_blueprint: Closure,
    pub get_modules: Closure,
    pub get_blueprints: Closure,
    pub add_provider: Closure,
    pub get_providers: Closure,
    pub get_interface: Closure,
    pub get_active_interfaces: Closure,
    pub identify: Closure,
}

impl HostClosures {
    pub fn descriptor(self) -> ClosureDescriptor {
        Arc::new(move || {
            let this = self.clone();
            Box::new(move |particle, args| this.route(particle, args))
        })
    }

    fn route(&self, particle: ParticleParameters, args: Vec<IValue>) -> Option<IValue> {
        let args = match Args::parse(args) {
            Ok(args) => args,
            Err(err) => {
                log::warn!("host function args parse error: {:?}", err);
                return ivalue_utils::error(json!(err.to_string()));
            }
        };
        log::info!(
            "Host function call {:?} {}",
            args.service_id,
            args.function_name
        );
        log::debug!("Host function call, args: {:#?}", args);

        // TODO: maybe error handling and conversion should happen here, so it is possible to log::warn errors
        #[rustfmt::skip]
        match (args.service_id.as_str(), args.function_name.as_str()) {
            ("dht", "resolve")         => (self.resolve)(args),
            ("dht", "neighborhood")    => (self.neighborhood)(args),
            ("dht", "add_provider")    => (self.add_provider)(args),
            ("dht", "get_providers")   => (self.get_providers)(args),

            ("srv", "create")          => (self.create_service)(args),
            ("srv", "get_interface")   => (self.get_interface)(args),
            ("srv", "get_interfaces")  => (self.get_active_interfaces)(args),

            ("dist", "add_module")     => (self.add_module)(args),
            ("dist", "add_blueprint")  => (self.add_blueprint)(args),
            ("dist", "get_modules")    => (self.get_modules)(args),
            ("dist", "get_blueprints") => (self.get_blueprints)(args),

            ("op", "identify") => (self.identify)(args),
            ("op", "identity") => ok(Array(args.function_args)),

            _ => (self.call_service)(particle, args),
        }
    }
}
