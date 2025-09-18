pub mod dto;
pub mod handler;
mod helpers;
pub mod manager;

pub use dto::{
    ErrorResponse, PluginContextType, PluginEnableRequest, PluginEnablementStatus,
    PluginInvocationPayload, PluginInvocationRequest, PluginMetadata, PluginRegistrationRequest,
    PluginUpdateRequest, ToolRegistrationResponse, ToolUpdateRequest,
};
pub(crate) use handler::{
    invoke_plugin, list_plugins, list_tools, register_plugin, register_tool, set_plugin_enablement,
    unregister_plugin, update_plugin, update_tool,
};
pub use manager::PluginManager;
