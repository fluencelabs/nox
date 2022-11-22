use derivative::Derivative;
use parking_lot::RwLock;
use std::collections::HashSet;
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
    pub fn new(
        spell_blueprint_id: String,
        all_spell_blueprint_ids: HashSet<String>,
        registered_spells: HashSet<String>,
    ) -> Self {
        Self {
            spell_blueprint_id,
            all_spell_blueprint_ids,
            registered_spells: Arc::new(RwLock::new(registered_spells)),
        }
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
