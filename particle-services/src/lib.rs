
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use fluence_app_service::{AppService, AppServiceError, IValue};

type XAppService = Arc<Mutex<AppService>>;

struct ParticleServices {
    services: Arc<RwLock<HashMap<String, XAppService>>>,
}

pub enum ServiceError {
    NoSuchService(String),
    AppServiceError(AppServiceError)
}

impl ParticleServices {
    pub fn create_service(&self) -> impl Fn(String) -> Result<String, AppServiceError> + Send + 'static {
        let services = self.services.clone();
        move |blueprint_id| {
            let vm = Self::create_vm(blueprint_id);
            match vm {
                Ok((service_id, vm)) => {
                    let vm = Arc::new(Mutex::new(vm));
                    services.write().insert(service_id.clone(), vm);
                    Ok(service_id)
                }
                Err(e) => Err(e)
            }
        }
    }

    pub fn call_service(&self) -> impl Fn(String, String, String, serde_json::Value) -> Result<Vec<IValue>, ServiceError> + Send + 'static {
        let services = self.services.clone();
        move |service_id, module, fname, args| {
            if let Some(vm) = services.read().get(&service_id) {
                let result = vm.lock().call(module, fname, args, <_>::default());
                result.map_err(ServiceError::AppServiceError)
            } else {
                Err(ServiceError::NoSuchService(service_id))
            }
        }
    }

    fn create_vm(blueprint_id: String) -> Result<(String, AppService), AppServiceError> {
        unimplemented!()
    }
}