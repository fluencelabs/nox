use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use derivative::Derivative;
use eyre::eyre;
use eyre::WrapErr;
use fluence_app_service::TomlMarineConfig;
use libp2p::PeerId;
use parking_lot::RwLock;

use particle_modules::{load_module_by_path, AddBlueprint, ModuleRepository};
use particle_services::ParticleAppServices;
use service_modules::{module_file_name, Dependency};

type WorkerId = PeerId;

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct SpellStorage {
    // The blueprint for the latest spell service.
    spell_blueprint_id: String,
    // All currently existing spells
    registered_spells: Arc<RwLock<HashSet<(String, WorkerId)>>>,
}

impl SpellStorage {
    pub fn create(
        spells_base_dir: &Path,
        services: &ParticleAppServices,
        modules: &ModuleRepository,
    ) -> eyre::Result<Self> {
        let spell_config_path = spell_config_path(spells_base_dir);
        let spell_blueprint_id = if spell_config_path.exists() {
            let cfg = TomlMarineConfig::load(spell_config_path)?;
            Self::load_spell_service(cfg, spells_base_dir, modules)?
        } else {
            Self::load_spell_service_from_crate(modules)?
        };
        let registered_spells = Self::restore_spells(services, modules);

        Ok(Self {
            spell_blueprint_id,
            registered_spells: Arc::new(RwLock::new(registered_spells)),
        })
    }

    fn load_spell_service_from_crate(modules: &ModuleRepository) -> eyre::Result<String> {
        use fluence_spell_distro::{modules as spell_modules, CONFIG};

        log::info!(
            "Spell service impl version: {}",
            fluence_spell_distro::VERSION
        );

        let spell_modules = spell_modules();
        let cfg: TomlMarineConfig = toml::from_slice(CONFIG)?;
        let mut hashes = Vec::new();
        for config in cfg.module {
            let name = config.name.clone();
            let module = spell_modules.get(name.as_str()).ok_or(eyre!(format!(
                "there's no module {} in the fluence_spell_distro::modules",
                config.name
            )))?;
            let hash = modules
                .add_module(module.to_vec(), config)
                .context(format!("adding spell module {name}"))?;
            hashes.push(Dependency::Hash(hash))
        }

        Ok(modules.add_blueprint(AddBlueprint::new("spell".to_string(), hashes))?)
    }

    fn load_spell_service(
        cfg: TomlMarineConfig,
        spells_base_dir: &Path,
        modules: &ModuleRepository,
    ) -> eyre::Result<String> {
        let mut hashes = Vec::new();
        for config in cfg.module {
            let load_from = config
                .load_from
                .clone()
                .unwrap_or(PathBuf::from(module_file_name(&Dependency::Name(
                    config.name.clone(),
                ))));
            let module_path = spells_base_dir.join(load_from);
            let module = load_module_by_path(module_path.as_ref())?;
            let hash = modules.add_module(module, config)?;
            hashes.push(Dependency::Hash(hash));
        }

        Ok(modules.add_blueprint(AddBlueprint::new("spell".to_string(), hashes))?)
    }

    fn restore_spells(
        services: &ParticleAppServices,
        modules: &ModuleRepository,
    ) -> HashSet<(String, PeerId)> {
        // Find blueprint ids of the already existing spells. They might be of older versions of the spell service.
        // These blueprint ids marked with name "spell" to differ from other blueprints.
        let all_spell_blueprint_ids = modules
            .get_blueprints()
            .into_iter()
            .filter(|blueprint| blueprint.name == "spell")
            .map(|x| x.id)
            .collect::<HashSet<_>>();
        // Find already created spells by corresponding blueprint_ids.
        services
            .list_services_with_info()
            .into_iter()
            .filter(|s| all_spell_blueprint_ids.contains(&s.blueprint_id))
            .map(|s| (s.id, s.worker_id))
            .collect::<_>()
    }

    pub fn get_registered_spells(&self) -> HashSet<(String, WorkerId)> {
        self.registered_spells.read().clone()
    }

    pub fn get_blueprint(&self) -> String {
        self.spell_blueprint_id.clone()
    }

    pub fn register_spell(&self, spell_id: String, worker_id: WorkerId) {
        let mut spells = self.registered_spells.write();
        spells.insert((spell_id, worker_id));
    }

    pub fn unregister_spell(&self, spell_id: &String) {
        self.registered_spells
            .write()
            .retain(|(id, _)| id != spell_id);
    }
}

fn spell_config_path(spells_base_dir: &Path) -> PathBuf {
    spells_base_dir.join("Config.toml")
}
