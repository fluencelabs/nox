use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use derivative::Derivative;
use eyre::eyre;
use eyre::WrapErr;
use fluence_app_service::TomlMarineConfig;
use parking_lot::RwLock;

use particle_modules::{load_module_by_path, AddBlueprint, ModuleRepository};
use particle_services::{ParticleAppServices, PeerScope};
use service_modules::module_file_name;

type SpellId = String;

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct SpellStorage {
    // The blueprint for the latest spell service. It's used to create new spells
    spell_blueprint_id: String,
    // All currently existing spells
    registered_spells: Arc<RwLock<HashMap<PeerScope, Vec<SpellId>>>>,
    scope_mapping: Arc<RwLock<HashMap<SpellId, PeerScope>>>,
}

impl SpellStorage {
    pub fn create(
        spells_base_dir: &Path,
        services: &ParticleAppServices,
        modules: &ModuleRepository,
    ) -> eyre::Result<(Self, String)> {
        let spell_config_path = spell_config_path(spells_base_dir);
        let (spell_blueprint_id, spell_version) = if spell_config_path.exists() {
            let cfg = TomlMarineConfig::load(spell_config_path)?;
            Self::load_spell_service(cfg, spells_base_dir, modules)?
        } else {
            Self::load_spell_service_from_crate(modules)?
        };
        let (registered_spells, scope_mapping) = Self::restore_spells(services);

        Ok((
            Self {
                spell_blueprint_id,
                registered_spells: Arc::new(RwLock::new(registered_spells)),
                scope_mapping: Arc::new(RwLock::new(scope_mapping)),
            },
            spell_version,
        ))
    }

    fn load_spell_service_from_crate(modules: &ModuleRepository) -> eyre::Result<(String, String)> {
        use fluence_spell_distro::{modules as spell_modules, CONFIG};

        log::info!(
            "Spell service impl version: {}",
            fluence_spell_distro::VERSION
        );

        let spell_modules = spell_modules();
        let cfg: TomlMarineConfig = toml_edit::de::from_slice(CONFIG)?;
        let mut hashes = Vec::new();
        for config in cfg.module {
            let name = config.name.clone();
            let module = spell_modules.get(name.as_str()).ok_or(eyre!(format!(
                "there's no module {} in the fluence_spell_distro::modules",
                config.name
            )))?;
            let module_hash = modules
                .add_module(config.name, module.to_vec())
                .context(format!("adding spell module {name}"))?;
            hashes.push(module_hash);
        }

        Ok((
            modules.add_blueprint(AddBlueprint::new("spell".to_string(), hashes))?,
            fluence_spell_distro::VERSION.to_string(),
        ))
    }

    fn load_spell_service(
        cfg: TomlMarineConfig,
        spells_base_dir: &Path,
        modules: &ModuleRepository,
    ) -> eyre::Result<(String, String)> {
        let mut hashes = Vec::new();
        let mut versions = Vec::new();
        for config in cfg.module {
            let load_from = config
                .load_from
                .clone()
                .unwrap_or(PathBuf::from(module_file_name(&config.name)));
            let module_path = spells_base_dir.join(load_from);
            let module = load_module_by_path(module_path.as_ref())?;
            let module_hash = modules.add_module(config.name, module)?;
            versions.push(String::from(&module_hash.to_string()[..8]));
            hashes.push(module_hash);
        }
        let spell_disk_version = format!("wasm hashes {}", versions.join(" "));
        Ok((
            modules.add_blueprint(AddBlueprint::new("spell".to_string(), hashes))?,
            spell_disk_version,
        ))
    }

    fn restore_spells(
        services: &ParticleAppServices,
    ) -> (
        HashMap<PeerScope, Vec<SpellId>>,
        HashMap<SpellId, PeerScope>,
    ) {
        let mut registered_spell: HashMap<PeerScope, Vec<SpellId>> = HashMap::new();
        let mut scope_mapping: HashMap<SpellId, PeerScope> = HashMap::new();

        let spell_services = services
            .list_services_all()
            .into_iter()
            .filter(|s| s.service_type.is_spell());

        for service in spell_services {
            let peer_scope = service.peer_scope;
            let spell_id = service.id;
            registered_spell
                .entry(peer_scope)
                .or_default()
                .push(spell_id.clone());
            scope_mapping.insert(spell_id, peer_scope);
        }

        (registered_spell, scope_mapping)
    }

    pub fn get_registered_spells(&self) -> HashMap<PeerScope, Vec<SpellId>> {
        self.registered_spells.read().clone()
    }

    pub fn get_scope(&self, spell_id: SpellId) -> Option<PeerScope> {
        self.scope_mapping.read().get(&spell_id).cloned()
    }

    pub fn get_registered_spells_by(&self, peer_scope: PeerScope) -> Vec<SpellId> {
        self.registered_spells
            .read()
            .get(&peer_scope)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_blueprint(&self) -> String {
        self.spell_blueprint_id.clone()
    }

    pub fn register_spell(&self, peer_scope: PeerScope, spell_id: String) {
        let mut spells = self.registered_spells.write();
        let mut scope_mapping = self.scope_mapping.write();
        spells.entry(peer_scope).or_default().push(spell_id.clone());
        scope_mapping.insert(spell_id, peer_scope);
    }

    pub fn unregister_spell(&self, peer_scope: PeerScope, spell_id: &str) {
        if let Some(spells) = self.registered_spells.write().get_mut(&peer_scope) {
            spells.retain(|sp_id| sp_id.ne(spell_id));
        }
    }
}

fn spell_config_path(spells_base_dir: &Path) -> PathBuf {
    spells_base_dir.join("Config.toml")
}
