/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::errors::StoreError;
use crate::traits::CanonicalStore;
use chrono::Utc;
use earmark_core::{ActorId, MigrationRecord, MigrationStrategy};

pub struct MigrationEngine<'a> {
    store: &'a dyn CanonicalStore,
    _registry: &'a dyn earmark_declarations::DeclarationRegistry,
}

impl<'a> MigrationEngine<'a> {
    pub fn new(
        store: &'a dyn CanonicalStore,
        registry: &'a dyn earmark_declarations::DeclarationRegistry,
    ) -> Self {
        Self {
            store,
            _registry: registry,
        }
    }

    pub fn verify_migration_path(&self, from: &str, to: &str) -> Result<(), StoreError> {
        // Implementation for version jump validation
        // For now, we just ensure to > from semantically or non-equal
        if from == to {
            return Err(StoreError::Generic(format!(
                "Migration source and target versions are identical: {}",
                from
            )));
        }
        Ok(())
    }

    pub fn apply_migration(
        &self,
        from_version: &str,
        to_version: &str,
        actor_id: ActorId,
        strategy: MigrationStrategy,
    ) -> Result<String, StoreError> {
        self.verify_migration_path(from_version, to_version)?;

        let migration_id = format!("mig_{}", Utc::now().timestamp());
        let record = MigrationRecord {
            migration_id: migration_id.clone(),
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            applied_by: actor_id,
            strategy,
            created_at: Utc::now(),
        };

        self.store.record_migration(record)?;
        Ok(migration_id)
    }
}
