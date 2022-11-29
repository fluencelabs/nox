use derivative::Derivative;
use parking_lot::RwLock;
use particle_modules::{load_module_by_path, AddBlueprint, ModuleRepository};

use fluence_app_service::TomlMarineConfig;
use particle_services::ParticleAppServices;
use service_modules::{Dependency, Hash};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct SpellStorage {
    // The blueprint for the latest spell service.
    spell_blueprint_id: String,
    // All blueprints that are used for spells
    all_spell_blueprint_ids: HashSet<String>,
    // All currently existing spells
    registered_spells: Arc<RwLock<HashSet<String>>>,
}

impl SpellStorage {
    pub fn create(
        spells_base_dir: PathBuf,
        services: &ParticleAppServices,
        modules: &ModuleRepository,
    ) -> eyre::Result<Self> {
        let spell_blueprint_id = SpellStorage::load_spell_service(modules, spells_base_dir)?;
        let (all_spell_blueprint_ids, registered_spells) =
            SpellStorage::restore_spells(services, modules);
        Ok(Self {
            spell_blueprint_id,
            all_spell_blueprint_ids,
            registered_spells: Arc::new(RwLock::new(registered_spells)),
        })
    }

    fn load_spell_service(
        modules: &ModuleRepository,
        spells_base_dir: PathBuf,
    ) -> eyre::Result<String> {
        let spell_cfg_path = spells_base_dir.to_owned().join("Config.toml");
        let cfg = TomlMarineConfig::load(spell_cfg_path).unwrap();
        let spell_cfg_prefix = PathBuf::from(spells_base_dir);
        let mut hashes = Vec::new();
        for config in cfg.module {
            let load_from = config
                .load_from
                .clone()
                .unwrap_or(PathBuf::from(config.name.to_owned() + ".wasm"));
            let module_path = spell_cfg_prefix.to_owned().join(load_from);
            let module = load_module_by_path(module_path.as_ref())?;
            let hash = modules.add_module(module, config)?;
            hashes.push(Dependency::Hash(Hash::from_hex(&hash).unwrap()));
        }

        Ok(modules.add_blueprint(AddBlueprint::new("spell".to_string(), hashes))?)
    }

    fn restore_spells(
        services: &ParticleAppServices,
        modules: &ModuleRepository,
    ) -> (HashSet<String>, HashSet<String>) {
        // Find blueprint ids of the already existing spells. They might be of older versions of the spell service.
        // These blueprint ids marked with name "spell" to differ from other blueprints.
        let all_spell_blueprint_ids = modules
            .get_blueprints()
            .into_iter()
            .filter(|blueprint| blueprint.name == "spell")
            .map(|x| x.id)
            .collect::<HashSet<_>>();
        // Find already created spells by corresponding blueprint_ids.
        let registered_spells = services
            .list_services_with_blueprints()
            .into_iter()
            .filter(|(_, blueprint)| all_spell_blueprint_ids.contains(blueprint))
            .map(|(id, _)| id)
            .collect::<_>();

        (all_spell_blueprint_ids, registered_spells)
    }

    pub fn get_registered_spells(&self) -> HashSet<String> {
        self.registered_spells.read().clone()
    }

    pub fn get_blueprint(&self) -> String {
        self.spell_blueprint_id.clone()
    }

    pub fn register_spell(&self, spell_id: String) {
        let mut spells = self.registered_spells.write();
        spells.insert(spell_id);
    }

    pub fn unregister_spell(&self, spell_id: &String) {
        self.registered_spells.write().retain(|id| id != spell_id);
    }

    pub fn has_spell(&self, spell_id: &String) -> bool {
        self.registered_spells.read().contains(spell_id)
    }
}
