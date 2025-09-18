use super::dto::RequestContext;

pub trait RequestContextExt {
    fn rate_limit_key(&self) -> String;
    fn label(&self) -> String;
}

impl RequestContextExt for RequestContext {
    fn rate_limit_key(&self) -> String {
        match self.context_type {
            crate::plugins::PluginContextType::User => {
                format!("user:{}", self.context_id)
            }
            crate::plugins::PluginContextType::Group => {
                format!("group:{}", self.context_id)
            }
        }
    }

    fn label(&self) -> String {
        match self.context_type {
            crate::plugins::PluginContextType::User => {
                format!("User {}", self.context_id)
            }
            crate::plugins::PluginContextType::Group => {
                format!("Group {}", self.context_id)
            }
        }
    }
}
