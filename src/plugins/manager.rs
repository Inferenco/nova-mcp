use std::collections::HashMap;
use std::str;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use chrono::Utc;
use jsonschema::JSONSchema;
use reqwest::Client;
use serde_json::Value;

use crate::context::{require_matching_context, validate_context_pair, RequestContext};
use crate::error::{NovaError, Result};

use super::dto::{
    GroupPluginRecord, PluginContextType, PluginEnableRequest, PluginEnablementStatus,
    PluginInvocationPayload, PluginInvocationRequest, PluginMetadata, PluginRegistrationRequest,
    PluginUpdateRequest, ToolRegistrationResponse, ToolUpdateRequest, UserPluginRecord,
};

pub struct PluginManager {
    plugins: RwLock<HashMap<u64, PluginMetadata>>,
    historical_plugins: RwLock<HashMap<String, PluginMetadata>>,
    user_tree: sled::Tree,
    group_tree: sled::Tree,
    sequence: AtomicU64,
    http_client: Client,
}

impl PluginManager {
    pub fn new(user_tree: sled::Tree, group_tree: sled::Tree) -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            historical_plugins: RwLock::new(HashMap::new()),
            user_tree,
            group_tree,
            sequence: AtomicU64::new(1),
            http_client: Client::new(),
        }
    }

    pub fn register_plugin(&self, request: PluginRegistrationRequest) -> Result<PluginMetadata> {
        self.register_plugin_internal(request, false)
    }

    pub fn register_tool(
        &self,
        request: PluginRegistrationRequest,
        owner_context: &RequestContext,
    ) -> Result<ToolRegistrationResponse> {
        let metadata = self.register_plugin_internal(request, true)?;

        let mut enable_request = PluginEnableRequest {
            context_type: owner_context.context_type.clone(),
            context_id: owner_context.context_id.clone(),
            plugin_id: metadata.plugin_id,
            enable: true,
            added_by: None,
        };

        if matches!(enable_request.context_type, PluginContextType::Group) {
            enable_request.added_by = Some(metadata.owner_id.clone());
        }

        self.set_enablement(enable_request)?;

        let fq_name = metadata.fully_qualified_name.clone().ok_or_else(|| {
            NovaError::internal("Expected fully qualified name for contextual tool")
        })?;

        Ok(ToolRegistrationResponse {
            plugin_id: metadata.plugin_id,
            fully_qualified_name: fq_name,
            version: metadata.version,
        })
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
        if let Some(input_schema) = update.input_schema {
            match input_schema {
                Some(schema) => {
                    Self::validate_schema(&schema)?;
                    plugin.input_schema = Some(schema);
                }
                None => {
                    plugin.input_schema = None;
                }
            }
        }
        if let Some(output_schema) = update.output_schema {
            match output_schema {
                Some(schema) => {
                    Self::validate_schema(&schema)?;
                    plugin.output_schema = Some(schema);
                }
                None => {
                    plugin.output_schema = None;
                }
            }
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

    pub fn list_plugins_for_context(
        &self,
        context_type: PluginContextType,
        context_id: &str,
    ) -> Result<Vec<PluginMetadata>> {
        let guard = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;

        let mut latest_by_name: HashMap<String, &PluginMetadata> = HashMap::new();

        for metadata in guard.values() {
            if metadata
                .context_type
                .as_ref()
                .map(|ct| ct == &context_type)
                .unwrap_or(false)
                && metadata
                    .context_id
                    .as_ref()
                    .map(|id| id == context_id)
                    .unwrap_or(false)
            {
                let entry = latest_by_name
                    .entry(metadata.name.clone())
                    .or_insert(metadata);
                if metadata.version > entry.version {
                    latest_by_name.insert(metadata.name.clone(), metadata);
                }
            }
        }

        Ok(latest_by_name
            .values()
            .map(|meta| (*meta).clone())
            .collect())
    }

    pub fn update_tool(
        &self,
        plugin_id: u64,
        request: ToolUpdateRequest,
        owner_context: &RequestContext,
    ) -> Result<PluginMetadata> {
        let mut guard = self
            .plugins
            .write()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;

        let plugin = guard
            .get_mut(&plugin_id)
            .ok_or_else(|| NovaError::plugin_not_found(plugin_id))?;

        let original_metadata = plugin.clone();

        let plugin_context = plugin
            .context_type
            .clone()
            .ok_or_else(|| NovaError::validation_error("Plugin is not context scoped"))?;
        let plugin_context_id = plugin
            .context_id
            .clone()
            .ok_or_else(|| NovaError::validation_error("Plugin is not context scoped"))?;

        let metadata_context = RequestContext {
            context_type: plugin_context,
            context_id: plugin_context_id.clone(),
        };

        require_matching_context(&metadata_context, owner_context)?;

        if let Some(name) = request.name.clone() {
            if name.trim().is_empty() {
                return Err(NovaError::validation_error("Tool name cannot be empty"));
            }
            plugin.name = name;
        }

        if let Some(description) = request.description.clone() {
            plugin.description = description;
        }

        if let Some(endpoint) = request.endpoint.clone() {
            if endpoint.trim().is_empty() {
                return Err(NovaError::validation_error(
                    "Plugin endpoint cannot be empty",
                ));
            }
            plugin.endpoint = endpoint;
        }

        if let Some(schema) = request.input_schema.clone() {
            Self::validate_schema(&schema)?;
            plugin.input_schema = Some(schema);
        }

        if let Some(output_schema) = request.output_schema.clone() {
            match output_schema {
                Some(schema) => {
                    Self::validate_schema(&schema)?;
                    plugin.output_schema = Some(schema);
                }
                None => plugin.output_schema = None,
            }
        }

        if let Some(icon_url) = request.icon_url.clone() {
            plugin.icon_url = icon_url;
        }

        if let Some(trust_level) = request.trust_level.clone() {
            plugin.trust_level = trust_level;
        }

        plugin.version = original_metadata.version.saturating_add(1);
        plugin.fully_qualified_name = Some(Self::format_fq_name(
            &metadata_context.context_type,
            &metadata_context.context_id,
            &plugin.name,
            plugin.version,
        ));

        if original_metadata.fully_qualified_name.is_some() {
            self.archive_metadata(original_metadata)?;
        }

        Ok(plugin.clone())
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

        if let Some(schema) = metadata.input_schema.as_ref() {
            Self::validate_against_schema(&arguments, schema)?;
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

        if let Some(schema) = metadata.output_schema.as_ref() {
            Self::validate_against_schema(&json, schema)?;
        }

        Ok(json)
    }

    pub fn get_plugin_by_fq_name(&self, fq_name: &str) -> Result<PluginMetadata> {
        let guard = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        if let Some(metadata) = guard
            .values()
            .find(|meta| meta.fully_qualified_name.as_deref() == Some(fq_name))
        {
            return Ok(metadata.clone());
        }
        drop(guard);

        let history = self
            .historical_plugins
            .read()
            .map_err(|_| NovaError::internal("Historical registry lock poisoned"))?;
        history
            .get(fq_name)
            .cloned()
            .ok_or_else(|| NovaError::validation_error(format!("Tool not found: {}", fq_name)))
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

    fn register_plugin_internal(
        &self,
        request: PluginRegistrationRequest,
        require_context: bool,
    ) -> Result<PluginMetadata> {
        let PluginRegistrationRequest {
            name,
            description,
            owner_id,
            scopes,
            endpoint,
            icon_url,
            trust_level,
            context_type,
            context_id,
            mut input_schema,
            mut output_schema,
            version,
        } = request;

        if name.trim().is_empty() {
            return Err(NovaError::validation_error("Plugin name cannot be empty"));
        }
        if endpoint.trim().is_empty() {
            return Err(NovaError::validation_error(
                "Plugin endpoint cannot be empty",
            ));
        }

        let context = match (context_type, context_id) {
            (Some(ct), Some(id)) => {
                validate_context_pair(&ct, &id)?;
                Some((ct, id))
            }
            (None, None) => {
                if require_context {
                    return Err(NovaError::validation_error(
                        "Context is required for tool registration",
                    ));
                }
                None
            }
            _ => {
                return Err(NovaError::validation_error(
                    "context_type and context_id must both be provided",
                ));
            }
        };

        match (&context, &mut input_schema) {
            (Some(_), Some(schema)) => Self::validate_schema(schema)?,
            (Some(_), None) => {
                return Err(NovaError::validation_error(
                    "input_schema is required for contextual tools",
                ))
            }
            (None, Some(schema)) => Self::validate_schema(schema)?,
            (None, None) => {}
        }

        if let Some(schema) = output_schema.as_mut() {
            Self::validate_schema(schema)?;
        }

        let version = if let Some(v) = version {
            if v == 0 {
                return Err(NovaError::validation_error(
                    "version must be greater than zero",
                ));
            }
            if let Some((ref ct, ref id)) = context {
                self.ensure_version_available(ct, id, &name, v)?;
            }
            v
        } else if let Some((ref ct, ref id)) = context {
            self.next_version(ct, id, &name)?
        } else {
            1
        };

        let plugin_id = self.sequence.fetch_add(1, Ordering::SeqCst);

        let fully_qualified_name = context
            .as_ref()
            .map(|(ct, id)| Self::format_fq_name(ct, id, &name, version));

        let metadata = PluginMetadata {
            plugin_id,
            name,
            description,
            owner_id,
            scopes,
            endpoint,
            icon_url,
            trust_level,
            context_type: context.as_ref().map(|(ct, _)| ct.clone()),
            context_id: context.as_ref().map(|(_, id)| id.clone()),
            input_schema,
            output_schema,
            version,
            fully_qualified_name,
        };

        let mut guard = self
            .plugins
            .write()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;

        guard.insert(plugin_id, metadata.clone());

        Ok(metadata)
    }

    fn archive_metadata(&self, metadata: PluginMetadata) -> Result<()> {
        if let Some(fq_name) = metadata.fully_qualified_name.clone() {
            let mut guard = self
                .historical_plugins
                .write()
                .map_err(|_| NovaError::internal("Historical registry lock poisoned"))?;
            guard.insert(fq_name, metadata);
        }
        Ok(())
    }

    fn ensure_version_available(
        &self,
        context_type: &PluginContextType,
        context_id: &str,
        name: &str,
        version: u32,
    ) -> Result<()> {
        let guard = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        let conflict = guard.values().any(|meta| {
            meta.context_type.as_ref() == Some(context_type)
                && meta.context_id.as_deref() == Some(context_id)
                && meta.name == name
                && meta.version == version
        });
        drop(guard);

        if conflict {
            return Err(NovaError::validation_error(
                "Tool version already exists for this context",
            ));
        }

        let history = self
            .historical_plugins
            .read()
            .map_err(|_| NovaError::internal("Historical registry lock poisoned"))?;
        let conflict = history.values().any(|meta| {
            meta.context_type.as_ref() == Some(context_type)
                && meta.context_id.as_deref() == Some(context_id)
                && meta.name == name
                && meta.version == version
        });

        if conflict {
            return Err(NovaError::validation_error(
                "Tool version already archived for this context",
            ));
        }

        Ok(())
    }

    fn next_version(
        &self,
        context_type: &PluginContextType,
        context_id: &str,
        name: &str,
    ) -> Result<u32> {
        let guard = self
            .plugins
            .read()
            .map_err(|_| NovaError::internal("Plugin registry lock poisoned"))?;
        let mut max_version = guard
            .values()
            .filter(|meta| {
                meta.context_type.as_ref() == Some(context_type)
                    && meta.context_id.as_deref() == Some(context_id)
                    && meta.name == name
            })
            .map(|meta| meta.version)
            .max()
            .unwrap_or(0);
        drop(guard);

        let history = self
            .historical_plugins
            .read()
            .map_err(|_| NovaError::internal("Historical registry lock poisoned"))?;
        if let Some(history_max) = history
            .values()
            .filter(|meta| {
                meta.context_type.as_ref() == Some(context_type)
                    && meta.context_id.as_deref() == Some(context_id)
                    && meta.name == name
            })
            .map(|meta| meta.version)
            .max()
        {
            if history_max > max_version {
                max_version = history_max;
            }
        }

        Ok(max_version + 1)
    }

    fn format_fq_name(
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

    fn validate_schema(schema: &Value) -> Result<()> {
        if !schema.is_object() {
            return Err(NovaError::validation_error("Schemas must be JSON objects"));
        }
        JSONSchema::compile(schema)
            .map_err(|err| NovaError::validation_error(format!("Invalid schema: {}", err)))?;
        Ok(())
    }

    fn validate_against_schema(value: &Value, schema: &Value) -> Result<()> {
        let compiled = JSONSchema::compile(schema)
            .map_err(|err| NovaError::validation_error(format!("Invalid schema: {}", err)))?;
        let result = compiled.validate(value);
        if let Err(errors) = result {
            let messages: Vec<String> = errors.map(|err| err.to_string()).collect();
            return Err(NovaError::validation_error(format!(
                "Payload does not match schema: {}",
                messages.join(", ")
            )));
        }
        Ok(())
    }
}
