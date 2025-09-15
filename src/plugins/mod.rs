pub mod dto;
pub mod handler;
mod helpers;
pub mod manager;

pub use dto::{
    ErrorResponse, PluginContextType, PluginEnableRequest, PluginEnablementStatus,
    PluginInvocationPayload, PluginInvocationRequest, PluginMetadata, PluginRegistrationRequest,
    PluginUpdateRequest,
};
pub(crate) use handler::{
    invoke_plugin, list_plugins, register_plugin, set_plugin_enablement, unregister_plugin,
    update_plugin,
};
pub use manager::PluginManager;
