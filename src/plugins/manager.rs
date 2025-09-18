use std::collections::HashMap;
use std::str;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use chrono::Utc;
use jsonschema::{Draft, JSONSchema};
use reqwest::Client;
use serde_json::Value;

use crate::error::{NovaError, Result};

use super::dto::{
    GroupPluginRecord, PluginContextType, PluginEnableRequest, PluginEnablementStatus,
    PluginInvocationPayload, PluginMetadata, PluginRegistrationRequest, PluginUpdateRequest,
    PluginVersionRecord, RequestContext, StoredPluginRecord, UserPluginRecord,
};

pub struct PluginManager {
    metadata_tree: sled::Tree,
    user_tree: sled::Tree,
    group_tree: sled::Tree,
    plugins: RwLock<HashMap<u64, StoredPluginRecord>>,
    fq_index: RwLock<HashMap<String, (u64, u32)>>,
    sequence: AtomicU64,
    http_client: Client,
}

impl PluginManager {
    pub fn new(
        metadata_tree: sled::Tree,
        user_tree: sled::Tree,
        group_tree: sled::Tree,
    ) -> Result<Self> {
        let (plugins, fq_index, next_id) = Self::load_plugins(&metadata_tree)?;
        Ok(Self {
            metadata_tree,
            user_tree,
            group_tree,
            plugins: RwLock::new(plugins),
            fq_index: RwLock::new(fq_index),
            sequence: AtomicU64::new(next_id),
            http_client: Client::new(),
        })
    }

    pub fn register_plugin(
        &self,
        context: &RequestContext,
        request: PluginRegistrationRequest,
    ) -> Result<PluginMetadata> {
        self.validate_registration(&request)?;
        let mut plugins = self
            .plugins
            .write()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;

        Self::ensure_unique_name(&plugins, context, &request.name)?;

        let plugin_id = self.sequence.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now().timestamp();
        let fq_name = Self::fq_name(
            &context.context_type,
            &context.context_id,
            &request.name,
            request.version,
        );

        self.ensure_unique_fq_name(&fq_name)?;

        let version_record = PluginVersionRecord {
            version: request.version,
            fq_name: fq_name.clone(),
            input_schema: request.input_schema.clone(),
            output_schema: request.output_schema.clone(),
            endpoint_url: request.endpoint_url.clone(),
            created_at: now,
        };

        let record = StoredPluginRecord {
            plugin_id,
            name: request.name,
            description: request.description,
            owner_id: request.owner_id,
            context_type: context.context_type.clone(),
            context_id: context.context_id.clone(),
            created_at: now,
            updated_at: now,
            versions: vec![version_record.clone()],
        };

        plugins.insert(plugin_id, record.clone());
        drop(plugins);

        self.persist_plugin(&record)?;
        self.insert_fq_mapping(&version_record, plugin_id);
        self.ensure_owner_enablement(&record)?;

        Ok(Self::to_metadata(&record, &version_record))
    }

    pub fn unregister_plugin(&self, context: &RequestContext, plugin_id: u64) -> Result<()> {
        let mut plugins = self
            .plugins
            .write()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;

        let record = plugins
            .get(&plugin_id)
            .cloned()
            .ok_or_else(|| NovaError::plugin_not_found(plugin_id))?;

        if record.context_type != context.context_type || record.context_id != context.context_id {
            return Err(NovaError::validation_error(
                "Only the owner context can delete a tool",
            ));
        }

        plugins.remove(&plugin_id);
        drop(plugins);

        self.metadata_tree
            .remove(plugin_id.to_be_bytes())
            .map_err(NovaError::from)?;
        self.metadata_tree.flush().map_err(NovaError::from)?;

        self.remove_fq_mappings(&record);
        self.clear_plugin_entries(plugin_id)?;
        Ok(())
    }

    pub fn update_plugin(
        &self,
        context: &RequestContext,
        plugin_id: u64,
        update: PluginUpdateRequest,
    ) -> Result<PluginMetadata> {
        self.validate_update(&update)?;
        let mut plugins = self
            .plugins
            .write()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;

        let record = plugins
            .get_mut(&plugin_id)
            .ok_or_else(|| NovaError::plugin_not_found(plugin_id))?;

        if record.context_type != context.context_type || record.context_id != context.context_id {
            return Err(NovaError::validation_error(
                "Only the owner context can update a tool",
            ));
        }

        let previous_version = record
            .versions
            .last()
            .ok_or_else(|| NovaError::internal("Plugin record has no versions"))?
            .clone();

        let new_version = previous_version.version + 1;
        let now = Utc::now().timestamp();
        let fq_name = Self::fq_name(
            &record.context_type,
            &record.context_id,
            &record.name,
            new_version,
        );

        self.ensure_unique_fq_name(&fq_name)?;

        if let Some(description) = update.description {
            record.description = description;
        }
        if let Some(owner_id) = update.owner_id {
            record.owner_id = Some(owner_id);
        }

        let input_schema = update
            .input_schema
            .unwrap_or(previous_version.input_schema.clone());
        let output_schema = match update.output_schema {
            Some(value) => value,
            None => previous_version.output_schema.clone(),
        };
        let endpoint_url = update
            .endpoint_url
            .unwrap_or(previous_version.endpoint_url.clone());

        let version_record = PluginVersionRecord {
            version: new_version,
            fq_name: fq_name.clone(),
            input_schema,
            output_schema,
            endpoint_url,
            created_at: now,
        };

        record.updated_at = now;
        record.versions.push(version_record.clone());

        let stored = record.clone();
        drop(plugins);

        self.persist_plugin(&stored)?;
        self.insert_fq_mapping(&version_record, plugin_id);

        Ok(Self::to_metadata(&stored, &version_record))
    }

    pub fn list_plugins_for_context(
        &self,
        context: &RequestContext,
    ) -> Result<Vec<PluginMetadata>> {
        let plugins = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;

        let mut result = Vec::new();
        for record in plugins.values() {
            let owner_match = record.context_type == context.context_type
                && record.context_id == context.context_id;
            let enabled = if owner_match {
                true
            } else {
                self.is_enabled(
                    record.plugin_id,
                    context.context_type.clone(),
                    &context.context_id,
                )?
            };

            if owner_match || enabled {
                if let Some(version) = record.versions.last() {
                    result.push(Self::to_metadata(record, version));
                }
            }
        }

        Ok(result)
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginMetadata>> {
        let plugins = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        let mut result = Vec::new();
        for record in plugins.values() {
            if let Some(version) = record.versions.last() {
                result.push(Self::to_metadata(record, version));
            }
        }
        Ok(result)
    }

    pub fn get_plugin(&self, plugin_id: u64) -> Result<PluginMetadata> {
        let plugins = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        let record = plugins
            .get(&plugin_id)
            .ok_or_else(|| NovaError::plugin_not_found(plugin_id))?;
        let version = record
            .versions
            .last()
            .ok_or_else(|| NovaError::internal("Plugin record has no versions"))?;
        Ok(Self::to_metadata(record, version))
    }

    pub fn get_plugin_by_fq_name(&self, fq_name: &str) -> Result<PluginMetadata> {
        let fq_index = self
            .fq_index
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        let (plugin_id, version) = fq_index
            .get(fq_name)
            .cloned()
            .ok_or_else(|| NovaError::api_error("Unknown tool"))?;
        drop(fq_index);

        let plugins = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        let record = plugins
            .get(&plugin_id)
            .ok_or_else(|| NovaError::plugin_not_found(plugin_id))?;
        let version = record
            .versions
            .iter()
            .find(|v| v.version == version)
            .ok_or_else(|| NovaError::internal("Version index out of sync"))?;
        Ok(Self::to_metadata(record, version))
    }

    pub fn set_enablement(&self, request: PluginEnableRequest) -> Result<PluginEnablementStatus> {
        self.ensure_plugin_exists(request.plugin_id)?;

        match request.context_type {
            PluginContextType::User => self.set_user_enablement(&request),
            PluginContextType::Group => self.set_group_enablement(&request),
        }
    }

    pub fn is_enabled(
        &self,
        plugin_id: u64,
        context_type: PluginContextType,
        context_id: &str,
    ) -> Result<bool> {
        match context_type {
            PluginContextType::User => self.read_user_enablement(context_id, plugin_id),
            PluginContextType::Group => self.read_group_enablement(context_id, plugin_id),
        }
    }

    pub async fn invoke_plugin(
        &self,
        metadata: &PluginMetadata,
        caller: &RequestContext,
        arguments: Value,
    ) -> Result<Value> {
        if caller.context_type == metadata.context_type && caller.context_id == metadata.context_id
        {
            // owner always enabled
        } else if !self.is_enabled(
            metadata.plugin_id,
            caller.context_type.clone(),
            &caller.context_id,
        )? {
            return Err(NovaError::plugin_not_enabled(
                metadata.plugin_id,
                Self::context_type_label(&caller.context_type),
                caller.context_id.clone(),
            ));
        }

        self.validate_instance(&metadata.input_schema, &arguments, "arguments")?;

        let payload = PluginInvocationPayload {
            context_type: caller.context_type.clone(),
            context_id: caller.context_id.clone(),
            arguments,
        };

        let response = self
            .http_client
            .post(&metadata.endpoint_url)
            .json(&payload)
            .send()
            .await
            .map_err(NovaError::from)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NovaError::api_error(format!(
                "Plugin endpoint returned {}: {}",
                status, body
            )));
        }

        let json = response.json().await.map_err(NovaError::from)?;
        if let Some(schema) = &metadata.output_schema {
            self.validate_instance(schema, &json, "response")?;
        }
        Ok(json)
    }

    fn validate_registration(&self, request: &PluginRegistrationRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            return Err(NovaError::validation_error("Plugin name cannot be empty"));
        }
        if request.description.trim().is_empty() {
            return Err(NovaError::validation_error(
                "Plugin description cannot be empty",
            ));
        }
        if request.endpoint_url.trim().is_empty() {
            return Err(NovaError::validation_error(
                "Plugin endpoint cannot be empty",
            ));
        }
        if !request.endpoint_url.starts_with("https://") {
            return Err(NovaError::validation_error(
                "Plugin endpoint must use HTTPS",
            ));
        }
        if request.version == 0 {
            return Err(NovaError::validation_error(
                "Version must be greater than or equal to 1",
            ));
        }
        self.validate_schema(&request.input_schema, "input_schema")?;
        if let Some(schema) = &request.output_schema {
            self.validate_schema(schema, "output_schema")?;
        }
        Ok(())
    }

    fn validate_update(&self, update: &PluginUpdateRequest) -> Result<()> {
        if let Some(schema) = &update.input_schema {
            self.validate_schema(schema, "input_schema")?;
        }
        if let Some(schema) = &update.output_schema {
            if let Some(value) = schema {
                self.validate_schema(value, "output_schema")?;
            }
        }
        if let Some(endpoint) = &update.endpoint_url {
            if endpoint.trim().is_empty() {
                return Err(NovaError::validation_error(
                    "Plugin endpoint cannot be empty",
                ));
            }
            if !endpoint.starts_with("https://") {
                return Err(NovaError::validation_error(
                    "Plugin endpoint must use HTTPS",
                ));
            }
        }
        Ok(())
    }

    fn validate_schema(&self, schema: &Value, label: &str) -> Result<()> {
        if !schema.is_object() {
            return Err(NovaError::validation_error(format!(
                "{} must be a JSON object",
                label
            )));
        }
        JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(schema)
            .map_err(|err| {
                NovaError::validation_error(format!(
                    "{} is not a valid JSON schema: {}",
                    label, err
                ))
            })?;
        Ok(())
    }

    fn validate_instance(&self, schema: &Value, instance: &Value, label: &str) -> Result<()> {
        let compiled = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(schema)
            .map_err(|err| {
                NovaError::validation_error(format!("{} schema compilation failed: {}", label, err))
            })?;
        if let Err(errors) = compiled.validate(instance) {
            let messages: Vec<String> = errors.map(|e| e.to_string()).collect();
            return Err(NovaError::validation_error(format!(
                "{} failed validation: {}",
                label,
                messages.join(", ")
            )));
        }
        Ok(())
    }

    fn ensure_unique_name(
        plugins: &HashMap<u64, StoredPluginRecord>,
        context: &RequestContext,
        name: &str,
    ) -> Result<()> {
        for record in plugins.values() {
            if record.context_type == context.context_type
                && record.context_id == context.context_id
                && record.name.eq_ignore_ascii_case(name)
            {
                return Err(NovaError::validation_error(
                    "A tool with this name already exists for the context",
                ));
            }
        }
        Ok(())
    }

    fn ensure_unique_fq_name(&self, fq_name: &str) -> Result<()> {
        let fq_index = self
            .fq_index
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        if fq_index.contains_key(fq_name) {
            return Err(NovaError::validation_error(
                "A tool with this version already exists",
            ));
        }
        Ok(())
    }

    fn insert_fq_mapping(&self, version: &PluginVersionRecord, plugin_id: u64) {
        if let Ok(mut map) = self.fq_index.write() {
            map.insert(version.fq_name.clone(), (plugin_id, version.version));
        }
    }

    fn remove_fq_mappings(&self, record: &StoredPluginRecord) {
        if let Ok(mut map) = self.fq_index.write() {
            for version in &record.versions {
                map.remove(&version.fq_name);
            }
        }
    }

    fn persist_plugin(&self, record: &StoredPluginRecord) -> Result<()> {
        let encoded = serde_json::to_vec(record).map_err(NovaError::from)?;
        self.metadata_tree
            .insert(record.plugin_id.to_be_bytes(), encoded)
            .map_err(NovaError::from)?;
        self.metadata_tree.flush().map_err(NovaError::from)?;
        Ok(())
    }

    fn ensure_owner_enablement(&self, record: &StoredPluginRecord) -> Result<()> {
        match record.context_type {
            PluginContextType::User => {
                let key = Self::context_key(&record.context_id, record.plugin_id);
                let now = Utc::now().timestamp();
                let user_record = UserPluginRecord {
                    enabled: true,
                    consent_ts: now,
                };
                let encoded = serde_json::to_vec(&user_record).map_err(NovaError::from)?;
                self.user_tree
                    .insert(key, encoded)
                    .map_err(NovaError::from)?;
                self.user_tree.flush().map_err(NovaError::from)?;
            }
            PluginContextType::Group => {
                let key = Self::context_key(&record.context_id, record.plugin_id);
                let now = Utc::now().timestamp();
                let group_record = GroupPluginRecord {
                    enabled: true,
                    added_by: None,
                    consent_ts: now,
                };
                let encoded = serde_json::to_vec(&group_record).map_err(NovaError::from)?;
                self.group_tree
                    .insert(key, encoded)
                    .map_err(NovaError::from)?;
                self.group_tree.flush().map_err(NovaError::from)?;
            }
        }
        Ok(())
    }

    fn ensure_plugin_exists(&self, plugin_id: u64) -> Result<()> {
        let plugins = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        if plugins.contains_key(&plugin_id) {
            Ok(())
        } else {
            Err(NovaError::plugin_not_found(plugin_id))
        }
    }

    fn load_plugins(
        tree: &sled::Tree,
    ) -> Result<(
        HashMap<u64, StoredPluginRecord>,
        HashMap<String, (u64, u32)>,
        u64,
    )> {
        let mut plugins = HashMap::new();
        let mut index = HashMap::new();
        let mut max_id = 0u64;
        for item in tree.iter() {
            let entry = item.map_err(NovaError::from)?;
            let id_bytes: [u8; 8] =
                entry.0.as_ref().try_into().map_err(|_| {
                    NovaError::internal("Failed to parse plugin id from metadata key")
                })?;
            let plugin_id = u64::from_be_bytes(id_bytes);
            let record: StoredPluginRecord =
                serde_json::from_slice(&entry.1).map_err(NovaError::from)?;
            for version in &record.versions {
                index.insert(version.fq_name.clone(), (plugin_id, version.version));
            }
            if plugin_id >= max_id {
                max_id = plugin_id + 1;
            }
            plugins.insert(plugin_id, record);
        }
        Ok((plugins, index, max_id.max(1)))
    }

    fn read_user_enablement(&self, context_id: &str, plugin_id: u64) -> Result<bool> {
        let key = Self::context_key(context_id, plugin_id);
        let value = self.user_tree.get(&key).map_err(NovaError::from)?;
        if let Some(bytes) = value {
            let record: UserPluginRecord =
                serde_json::from_slice(&bytes).map_err(NovaError::from)?;
            Ok(record.enabled)
        } else {
            Ok(false)
        }
    }

    fn read_group_enablement(&self, context_id: &str, plugin_id: u64) -> Result<bool> {
        let key = Self::context_key(context_id, plugin_id);
        let value = self.group_tree.get(&key).map_err(NovaError::from)?;
        if let Some(bytes) = value {
            let record: GroupPluginRecord =
                serde_json::from_slice(&bytes).map_err(NovaError::from)?;
            Ok(record.enabled)
        } else {
            Ok(false)
        }
    }

    fn set_user_enablement(&self, request: &PluginEnableRequest) -> Result<PluginEnablementStatus> {
        let key = Self::context_key(&request.context_id, request.plugin_id);
        let now = Utc::now().timestamp();
        let existing = self.user_tree.get(&key).map_err(NovaError::from)?;

        let mut record = if let Some(value) = existing {
            serde_json::from_slice::<UserPluginRecord>(&value).map_err(NovaError::from)?
        } else {
            UserPluginRecord {
                enabled: false,
                consent_ts: now,
            }
        };

        if request.enable {
            record.enabled = true;
            record.consent_ts = now;
        } else {
            record.enabled = false;
        }

        let encoded = serde_json::to_vec(&record).map_err(NovaError::from)?;
        self.user_tree
            .insert(key, encoded)
            .map_err(NovaError::from)?;
        self.user_tree.flush().map_err(NovaError::from)?;

        Ok(PluginEnablementStatus {
            context_type: PluginContextType::User,
            context_id: request.context_id.clone(),
            plugin_id: request.plugin_id,
            enabled: record.enabled,
            consent_ts: record.consent_ts,
            added_by: None,
        })
    }

    fn set_group_enablement(
        &self,
        request: &PluginEnableRequest,
    ) -> Result<PluginEnablementStatus> {
        let key = Self::context_key(&request.context_id, request.plugin_id);
        let now = Utc::now().timestamp();
        let existing = self.group_tree.get(&key).map_err(NovaError::from)?;

        let mut record = if let Some(value) = existing {
            serde_json::from_slice::<GroupPluginRecord>(&value).map_err(NovaError::from)?
        } else {
            GroupPluginRecord {
                enabled: false,
                added_by: None,
                consent_ts: now,
            }
        };

        if request.enable {
            let added_by = request.added_by.clone().ok_or_else(|| {
                NovaError::validation_error(
                    "added_by is required when enabling a plugin for a group",
                )
            })?;
            record.enabled = true;
            record.added_by = Some(added_by);
            record.consent_ts = now;
        } else {
            record.enabled = false;
        }

        let encoded = serde_json::to_vec(&record).map_err(NovaError::from)?;
        self.group_tree
            .insert(key, encoded)
            .map_err(NovaError::from)?;
        self.group_tree.flush().map_err(NovaError::from)?;

        Ok(PluginEnablementStatus {
            context_type: PluginContextType::Group,
            context_id: request.context_id.clone(),
            plugin_id: request.plugin_id,
            enabled: record.enabled,
            consent_ts: record.consent_ts,
            added_by: record.added_by.clone(),
        })
    }

    fn clear_plugin_entries(&self, plugin_id: u64) -> Result<()> {
        self.clear_entries_for_tree(&self.user_tree, plugin_id)?;
        self.clear_entries_for_tree(&self.group_tree, plugin_id)?;
        Ok(())
    }

    fn clear_entries_for_tree(&self, tree: &sled::Tree, plugin_id: u64) -> Result<()> {
        let mut keys_to_remove = Vec::new();
        for item in tree.iter() {
            let entry = item.map_err(NovaError::from)?;
            let key_bytes = entry.0.to_vec();
            if Self::matches_plugin(&key_bytes, plugin_id)? {
                keys_to_remove.push(key_bytes);
            }
        }

        for key in keys_to_remove {
            tree.remove(key).map_err(NovaError::from)?;
        }
        tree.flush().map_err(NovaError::from)?;
        Ok(())
    }

    fn matches_plugin(key: &[u8], plugin_id: u64) -> Result<bool> {
        let key_str = str::from_utf8(key).map_err(|e| {
            NovaError::internal(format!("Failed to parse sled key as UTF-8: {}", e))
        })?;
        if let Some((_context, id_str)) = key_str.rsplit_once('|') {
            let parsed = id_str.parse::<u64>().map_err(|e| {
                NovaError::internal(format!(
                    "Failed to parse plugin id from key '{}': {}",
                    key_str, e
                ))
            })?;
            Ok(parsed == plugin_id)
        } else {
            Ok(false)
        }
    }

    fn context_key(context_id: &str, plugin_id: u64) -> Vec<u8> {
        format!("{}|{}", context_id, plugin_id).into_bytes()
    }

    fn fq_name(
        context_type: &PluginContextType,
        context_id: &str,
        name: &str,
        version: u32,
    ) -> String {
        match context_type {
            PluginContextType::User => {
                format!("user_{}_{}_v{}", context_id, name, version)
            }
            PluginContextType::Group => {
                format!("group_{}_{}_v{}", context_id, name, version)
            }
        }
    }

    fn context_type_label(context_type: &PluginContextType) -> String {
        match context_type {
            PluginContextType::User => "user".to_string(),
            PluginContextType::Group => "group".to_string(),
        }
    }

    fn to_metadata(record: &StoredPluginRecord, version: &PluginVersionRecord) -> PluginMetadata {
        PluginMetadata {
            plugin_id: record.plugin_id,
            name: record.name.clone(),
            description: record.description.clone(),
            owner_id: record.owner_id.clone(),
            context_type: record.context_type.clone(),
            context_id: record.context_id.clone(),
            fq_name: version.fq_name.clone(),
            version: version.version,
            input_schema: version.input_schema.clone(),
            output_schema: version.output_schema.clone(),
            endpoint_url: version.endpoint_url.clone(),
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}
