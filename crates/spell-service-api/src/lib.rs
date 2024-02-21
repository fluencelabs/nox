use fluence_libp2p::PeerId;
use fluence_spell_dtos::trigger_config::{TriggerConfig, TriggerConfigValue};
use fluence_spell_dtos::value::{ScriptValue, SpellValueT, StringValue, U32Value, UnitValue};
use particle_execution::{FunctionOutcome, ParticleParams};
use particle_services::{ParticleAppServices, PeerScope};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("Spell {spell_id} not found (function {function_name})")]
    ServiceNotFound {
        spell_id: String,
        function_name: String,
    },
    #[error("Call {spell_id}.{function_name} didn't return any result")]
    EmptyResult {
        spell_id: String,
        function_name: String,
    },
    #[error("Error while calling {spell_id}.{function_name}: {reason}")]
    OtherError {
        spell_id: String,
        function_name: String,
        reason: String,
    },
    #[error("Result of the call {spell_id}.{function_name} cannot be parsed to the {target_type} type: {reason}")]
    ResultParseError {
        spell_id: String,
        function_name: String,
        target_type: &'static str,
        reason: String,
    },
    #[error("Call {spell_id}.{function_name} executed with the error: {reason}")]
    ExecutionError {
        spell_id: String,
        function_name: String,
        reason: String,
    },
}

struct Function {
    name: &'static str,
    args: Vec<Value>,
}

#[derive(Clone)]
pub struct CallParams {
    // Who initiated the call
    init_peer_id: PeerId,
    // Worker ID where the spell is installed
    peer_scope: PeerScope,
    // Spell ID
    spell_id: String,
    //
    particle_id: Option<String>,
    // Timeout for spell execution
    ttl: Duration,
}

impl CallParams {
    pub fn new(
        init_peer_id: PeerId,
        peer_scope: PeerScope,
        spell_id: String,
        particle_id: Option<String>,
        ttl: Duration,
    ) -> Self {
        Self {
            init_peer_id,
            peer_scope,
            spell_id,
            particle_id,
            ttl,
        }
    }
    pub fn from(spell_id: String, params: ParticleParams) -> Self {
        Self {
            init_peer_id: params.init_peer_id,
            peer_scope: params.peer_scope,
            spell_id,
            particle_id: Some(params.id),
            ttl: Duration::from_millis(params.ttl as u64),
        }
    }

    pub fn local(
        peer_scope: PeerScope,
        spell_id: String,
        init_peer_id: PeerId,
        ttl: Duration,
    ) -> Self {
        Self {
            init_peer_id,
            peer_scope,
            spell_id,
            particle_id: None,
            ttl,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpellServiceApi {
    services: ParticleAppServices,
}

impl SpellServiceApi {
    pub fn new(services: ParticleAppServices) -> Self {
        Self { services }
    }

    pub fn set_script(&self, params: CallParams, script: String) -> Result<(), CallError> {
        let function = Function {
            name: "set_script",
            args: vec![json!(script)],
        };
        let _ = self.call::<UnitValue>(params, function)?;
        Ok(())
    }

    pub fn get_script(&self, params: CallParams) -> Result<String, CallError> {
        let function = Function {
            name: "get_script",
            args: vec![],
        };
        let script_value = self.call::<ScriptValue>(params, function)?;
        Ok(script_value.value)
    }
    pub fn set_trigger_config(
        &self,
        params: CallParams,
        config: TriggerConfig,
    ) -> Result<(), CallError> {
        let function = Function {
            name: "set_trigger_config",
            args: vec![json!(config)],
        };
        let _ = self.call::<UnitValue>(params, function)?;
        Ok(())
    }

    pub fn get_trigger_config(&self, params: CallParams) -> Result<TriggerConfig, CallError> {
        let function = Function {
            name: "get_trigger_config",
            args: vec![],
        };
        let trigger_config_value = self.call::<TriggerConfigValue>(params, function)?;
        Ok(trigger_config_value.config)
    }

    // TODO: use `Map<String, Value>` for init_data instead of `Value`
    pub fn update_kv(&self, params: CallParams, kv_data: Value) -> Result<(), CallError> {
        let function = Function {
            name: "set_json_fields",
            args: vec![json!(kv_data.to_string())],
        };
        let _ = self.call::<UnitValue>(params, function)?;
        Ok(())
    }

    pub fn get_string(&self, params: CallParams, key: String) -> Result<Option<String>, CallError> {
        let function = Function {
            name: "get_string",
            args: vec![json!(key)],
        };
        let result = self.call::<StringValue>(params, function)?;
        Ok((!result.absent).then_some(result.value))
    }

    pub fn set_string(
        &self,
        params: CallParams,
        key: String,
        value: String,
    ) -> Result<(), CallError> {
        let function = Function {
            name: "set_string",
            args: vec![json!(key), json!(value)],
        };
        let _ = self.call::<UnitValue>(params, function)?;
        Ok(())
    }

    /// Load the counter (how many times the spell was run)
    pub fn get_counter(&self, params: CallParams) -> Result<Option<u32>, CallError> {
        let function = Function {
            name: "get_u32",
            args: vec![json!("hw_counter")],
        };
        let result = self.call::<U32Value>(params, function)?;
        Ok((!result.absent).then_some(result.value))
    }

    /// Update the counter (how many times the spell was run)
    /// TODO: permission check here or not?
    pub fn set_counter(&self, params: CallParams, counter: u32) -> Result<(), CallError> {
        let function = Function {
            name: "set_u32",
            args: vec![json!("hw_counter"), json!(counter)],
        };
        let _ = self.call::<UnitValue>(params, function)?;

        Ok(())
    }

    pub fn set_trigger_event(&self, params: CallParams, event: String) -> Result<(), CallError> {
        self.set_string(params, "hw_trigger".to_string(), event)
    }

    pub fn store_error(&self, params: CallParams, args: Vec<Value>) -> Result<(), CallError> {
        let function = Function {
            name: "store_error",
            args,
        };
        let _ = self.call::<UnitValue>(params, function)?;
        Ok(())
    }

    fn call<T>(&self, params: CallParams, function: Function) -> Result<T, CallError>
    where
        T: DeserializeOwned + SpellValueT,
    {
        use CallError::*;
        let spell_id = params.spell_id;
        let result = self.services.call_function(
            params.peer_scope,
            &spell_id,
            function.name,
            function.args,
            params.particle_id,
            params.init_peer_id,
            params.ttl,
        );
        match result {
            FunctionOutcome::NotDefined { .. } => Err(ServiceNotFound {
                spell_id,
                function_name: function.name.to_string(),
            }),
            FunctionOutcome::Empty => Err(EmptyResult {
                spell_id,
                function_name: function.name.to_string(),
            }),
            FunctionOutcome::Err(err) => Err(OtherError {
                spell_id,
                function_name: function.name.to_string(),
                reason: err.to_string(),
            }),
            FunctionOutcome::Ok(value) => match serde_json::from_value::<T>(value) {
                Ok(result) if result.is_success() => Ok(result),
                Ok(result) => Err(ExecutionError {
                    spell_id,
                    function_name: function.name.to_string(),
                    reason: result.take_error(),
                }),
                Err(e) => Err(ResultParseError {
                    spell_id,
                    function_name: function.name.to_string(),
                    target_type: std::any::type_name::<T>(),
                    reason: e.to_string(),
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use particle_services::{ParticleAppServices, PeerScope, ServiceType};

    use fluence_libp2p::PeerId;
    use libp2p_identity::Keypair;
    use tempdir::TempDir;

    use particle_modules::ModuleRepository;
    use server_config::ServicesConfig;

    use fluence_keypair::KeyPair;
    use fluence_spell_dtos::trigger_config::TriggerConfig;
    use fluence_spell_dtos::value::*;
    use maplit::hashmap;
    use serde_json::json;
    use std::time::Duration;
    use workers::{DummyCoreManager, KeyStorage, PeerScopes, Workers};

    use crate::{CallParams, SpellServiceApi};

    const TTL: Duration = Duration::from_millis(100000);

    fn create_pid() -> PeerId {
        let keypair = Keypair::generate_ed25519();

        PeerId::from(keypair.public())
    }

    async fn create_pas(
        management_pid: PeerId,
        base_dir: PathBuf,
    ) -> (ParticleAppServices, ModuleRepository, PeerId) {
        let persistent_dir = base_dir.join("persistent");
        let ephemeral_dir = base_dir.join("ephemeral");
        let root_key_pair = Keypair::generate_ed25519();
        let vault_dir = ephemeral_dir.join("..").join("vault");
        let keypairs_dir = persistent_dir.join("..").join("keypairs");
        let workers_dir = persistent_dir.join("..").join("workers");

        let root_key_pair: KeyPair = root_key_pair.clone().into();

        let key_storage = KeyStorage::from_path(keypairs_dir.clone(), root_key_pair.clone())
            .await
            .expect("Could not load key storage");

        let key_storage = Arc::new(key_storage);

        let core_manager = Arc::new(DummyCoreManager::default().into());

        let scope = PeerScopes::new(
            root_key_pair.get_peer_id(),
            management_pid,
            root_key_pair.get_peer_id(),
            key_storage.clone(),
        );

        let workers = Workers::from_path(workers_dir.clone(), key_storage, core_manager)
            .await
            .expect("Could not load worker registry");

        let workers = Arc::new(workers);

        let service_memory_limit = server_config::default_service_memory_limit();
        let config = ServicesConfig::new(
            root_key_pair.get_peer_id(),
            persistent_dir,
            ephemeral_dir,
            vault_dir,
            HashMap::new(),
            management_pid,
            root_key_pair.get_peer_id(),
            Some(service_memory_limit),
            Default::default(),
        )
        .unwrap();

        let repo = ModuleRepository::new(
            &config.modules_dir,
            &config.blueprint_dir,
            Default::default(),
        );

        let pas = ParticleAppServices::new(
            config,
            repo.clone(),
            None,
            None,
            workers.clone(),
            scope.clone(),
        );
        (pas, repo, root_key_pair.get_peer_id())
    }

    async fn create_spell(
        pas: &ParticleAppServices,
        blueprint_id: String,
        owner_id: PeerId,
    ) -> Result<String, String> {
        pas.create_service(PeerScope::Host, ServiceType::Spell, blueprint_id, owner_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn setup() -> (SpellServiceApi, CallParams) {
        let base_dir = TempDir::new("test3").unwrap();
        let management_pid = create_pid();
        let (pas, repo, local_pid) = create_pas(management_pid, base_dir.into_path()).await;

        let api = SpellServiceApi::new(pas.clone());
        let (storage, _) = spell_storage::SpellStorage::create(Path::new(""), &pas, &repo).unwrap();
        let spell_service_blueprint_id = storage.get_blueprint();
        let spell_id = create_spell(&pas, spell_service_blueprint_id, local_pid)
            .await
            .unwrap();
        let params = CallParams::local(PeerScope::Host, spell_id, local_pid, TTL);
        (api, params)
    }

    #[tokio::test]
    async fn test_counter() {
        let (api, params) = setup().await;
        let result1 = api.get_counter(params.clone());
        assert!(
            result1.is_ok(),
            "must be able to get a counter of an empty spell"
        );
        assert_eq!(
            result1.unwrap(),
            None,
            "the counter of an empty spell must be zero"
        );
        let new_counter = 7;
        let result2 = api.set_counter(params.clone(), new_counter);
        assert!(
            result2.is_ok(),
            "must be able to set a counter of an empty spell: {:?}",
            result2.unwrap_err()
        );
        let result3 = api.get_counter(params);
        assert!(
            result3.is_ok(),
            "must be able to get a counter of an empty spell again"
        );
        assert_eq!(
            result3.unwrap(),
            Some(new_counter),
            "must be able to load an updated counter"
        );
    }

    #[tokio::test]
    async fn test_script() {
        let (api, params) = setup().await;
        let script_original = "(noop)".to_string();
        let result1 = api.set_script(params.clone(), script_original.clone());
        assert!(result1.is_ok(), "must be able to update script");
        let script = api.get_script(params);
        assert!(script.is_ok(), "must be able to load script");
        assert_eq!(script.unwrap(), script_original, "scripts must be equal");
    }

    #[tokio::test]
    async fn test_trigger_config() {
        let (api, params) = setup().await;
        let trigger_config_original = TriggerConfig::default();
        let result1 = api.set_trigger_config(params.clone(), trigger_config_original.clone());
        assert!(result1.is_ok(), "must be able to set trigger config");
        let result2 = api.get_trigger_config(params);
        assert!(result2.is_ok(), "must be able to get trigger config");
        assert_eq!(
            result2.unwrap(),
            trigger_config_original,
            "trigger configs must be equal"
        );
    }

    #[tokio::test]
    async fn test_kv() {
        let (api, host_params) = setup().await;
        let init_data = hashmap! {
            "a1" => json!(1),
            "b1" => json!("test"),
        };
        let result1 = api.update_kv(host_params.clone(), json!(init_data));
        assert!(
            !result1.is_ok(),
            "must NOT be able to update kv without h/hw key prefixes calling from host"
        );

        let params = CallParams::new(
            host_params.init_peer_id,
            PeerScope::Host,
            host_params.spell_id.clone(),
            Some(format!("spell_{}_0", host_params.spell_id)),
            host_params.ttl,
        );

        let result1 = api.update_kv(params.clone(), json!(init_data));
        assert!(
            result1.is_ok(),
            "must be able to update kv calling as a spell"
        );

        let result = api.get_string(params.clone(), "a1".to_string());
        assert!(result.is_ok(), "must be able to add get_string");
        assert_eq!(
            result.unwrap().unwrap(),
            "1",
            "must be able to add get_string"
        );

        let result = api.get_string(params, "b1".to_string());
        assert!(result.is_ok(), "must be able to add get_string");
        assert_eq!(
            result.unwrap().unwrap(),
            "\"test\"",
            "must be able to add get_string"
        );
    }

    #[tokio::test]
    async fn test_trigger_event() {
        let (api, params) = setup().await;
        let trigger_event = json!({
            "peer": json!([]),
            "timer": vec![json!({
                "timestamp": 1
            })]
        });
        let result = api.set_trigger_event(params.clone(), trigger_event.to_string());
        assert!(result.is_ok(), "must be able to set trigger event");

        let function = super::Function {
            name: "get_string",
            args: vec![json!("hw_trigger")],
        };
        let result = api.call::<StringValue>(params, function);
        assert!(result.is_ok(), "must be able to add get_string");
        let trigger_event_read: Result<serde_json::Value, _> =
            serde_json::from_str(&result.unwrap().value);
        assert!(
            trigger_event_read.is_ok(),
            "read trigger event must be parsable"
        );
        assert_eq!(
            trigger_event_read.unwrap(),
            trigger_event,
            "read trigger event must be equal to the original one"
        );
    }
}
