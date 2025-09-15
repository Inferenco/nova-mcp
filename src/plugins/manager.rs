use std::collections::HashMap;
use std::str;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use chrono::Utc;
use reqwest::Client;

use crate::error::{NovaError, Result};

use super::dto::{
    GroupPluginRecord, PluginContextType, PluginEnableRequest, PluginEnablementStatus,
    PluginInvocationPayload, PluginInvocationRequest, PluginMetadata, PluginRegistrationRequest,
    PluginUpdateRequest, UserPluginRecord,
};

pub struct PluginManager {
    plugins: RwLock<HashMap<u64, PluginMetadata>>,
    user_tree: sled::Tree,
    group_tree: sled::Tree,
    sequence: AtomicU64,
    http_client: Client,
}

impl PluginManager {
    pub fn new(user_tree: sled::Tree, group_tree: sled::Tree) -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            user_tree,
            group_tree,
            sequence: AtomicU64::new(1),
            http_client: Client::new(),
        }
    }

    pub fn register_plugin(&self, request: PluginRegistrationRequest) -> Result<PluginMetadata> {
        if request.name.trim().is_empty() {
            return Err(NovaError::validation_error("Plugin name cannot be empty"));
        }
        if request.endpoint.trim().is_empty() {
            return Err(NovaError::validation_error(
                "Plugin endpoint cannot be empty",
            ));
        }

        let plugin_id = self.sequence.fetch_add(1, Ordering::SeqCst);
        let metadata = PluginMetadata {
            plugin_id,
            name: request.name,
            description: request.description,
            owner_id: request.owner_id,
            scopes: request.scopes,
            endpoint: request.endpoint,
            icon_url: request.icon_url,
            trust_level: request.trust_level,
        };

        let mut guard = self
            .plugins
            .write()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        guard.insert(plugin_id, metadata.clone());

        Ok(metadata)
    }

    pub fn unregister_plugin(&self, plugin_id: u64) -> Result<()> {
        let mut guard = self
            .plugins
            .write()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        let removed = guard.remove(&plugin_id);
        drop(guard);

        if removed.is_none() {
            return Err(NovaError::plugin_not_found(plugin_id));
        }

        self.clear_plugin_entries(plugin_id)?;
        Ok(())
    }

    pub fn update_plugin(
        &self,
        plugin_id: u64,
        update: PluginUpdateRequest,
    ) -> Result<PluginMetadata> {
        let mut guard = self
            .plugins
            .write()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        let plugin = guard
            .get_mut(&plugin_id)
            .ok_or_else(|| NovaError::plugin_not_found(plugin_id))?;

        if let Some(name) = update.name {
            if name.trim().is_empty() {
                return Err(NovaError::validation_error("Plugin name cannot be empty"));
            }
            plugin.name = name;
        }
        if let Some(description) = update.description {
            plugin.description = description;
        }
        if let Some(owner_id) = update.owner_id {
            plugin.owner_id = owner_id;
        }
        if let Some(scopes) = update.scopes {
            plugin.scopes = scopes;
        }
        if let Some(endpoint) = update.endpoint {
            if endpoint.trim().is_empty() {
                return Err(NovaError::validation_error(
                    "Plugin endpoint cannot be empty",
                ));
            }
            plugin.endpoint = endpoint;
        }
        if let Some(icon_url) = update.icon_url {
            plugin.icon_url = icon_url;
        }
        if let Some(trust_level) = update.trust_level {
            plugin.trust_level = trust_level;
        }

        Ok(plugin.clone())
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginMetadata>> {
        let guard = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        Ok(guard.values().cloned().collect())
    }

    pub fn get_plugin(&self, plugin_id: u64) -> Result<PluginMetadata> {
        let guard = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        guard
            .get(&plugin_id)
            .cloned()
            .ok_or_else(|| NovaError::plugin_not_found(plugin_id))
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
        plugin_id: u64,
        request: PluginInvocationRequest,
    ) -> Result<serde_json::Value> {
        let metadata = self.get_plugin(plugin_id)?;
        let PluginInvocationRequest {
            context_type,
            context_id,
            arguments,
        } = request;

        if !self.is_enabled(plugin_id, context_type.clone(), &context_id)? {
            return Err(NovaError::plugin_not_enabled(
                plugin_id,
                Self::context_type_label(&context_type),
                context_id,
            ));
        }

        let payload = PluginInvocationPayload {
            context_type,
            context_id,
            arguments,
        };

        let response = self
            .http_client
            .post(&metadata.endpoint)
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
        Ok(json)
    }

    fn ensure_plugin_exists(&self, plugin_id: u64) -> Result<()> {
        let guard = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        if guard.contains_key(&plugin_id) {
            Ok(())
        } else {
            Err(NovaError::plugin_not_found(plugin_id))
        }
    }

    fn set_user_enablement(&self, request: &PluginEnableRequest) -> Result<PluginEnablementStatus> {
        let key = Self::context_key(&request.context_id, request.plugin_id);
        let now = Utc::now().timestamp();
        let existing = self.user_tree.get(&key).map_err(NovaError::from)?;

        let mut record = if let Some(value) = existing {
            serde_json::from_slice::<UserPluginRecord>(&value).map_err(|e| {
                NovaError::internal(format!("Failed to parse user plugin record: {}", e))
            })?
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

        let encoded = serde_json::to_vec(&record).map_err(|e| {
            NovaError::internal(format!("Failed to encode user plugin record: {}", e))
        })?;
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
            serde_json::from_slice::<GroupPluginRecord>(&value).map_err(|e| {
                NovaError::internal(format!("Failed to parse group plugin record: {}", e))
            })?
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

        let encoded = serde_json::to_vec(&record).map_err(|e| {
            NovaError::internal(format!("Failed to encode group plugin record: {}", e))
        })?;
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

    fn read_user_enablement(&self, context_id: &str, plugin_id: u64) -> Result<bool> {
        let key = Self::context_key(context_id, plugin_id);
        let value = self.user_tree.get(&key).map_err(NovaError::from)?;
        if let Some(bytes) = value {
            let record: UserPluginRecord = serde_json::from_slice(&bytes).map_err(|e| {
                NovaError::internal(format!("Failed to parse user plugin record: {}", e))
            })?;
            Ok(record.enabled)
        } else {
            Ok(false)
        }
    }

    fn read_group_enablement(&self, context_id: &str, plugin_id: u64) -> Result<bool> {
        let key = Self::context_key(context_id, plugin_id);
        let value = self.group_tree.get(&key).map_err(NovaError::from)?;
        if let Some(bytes) = value {
            let record: GroupPluginRecord = serde_json::from_slice(&bytes).map_err(|e| {
                NovaError::internal(format!("Failed to parse group plugin record: {}", e))
            })?;
            Ok(record.enabled)
        } else {
            Ok(false)
        }
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

    fn context_type_label(context_type: &PluginContextType) -> String {
        match context_type {
            PluginContextType::User => "user".to_string(),
            PluginContextType::Group => "group".to_string(),
        }
    }
}
