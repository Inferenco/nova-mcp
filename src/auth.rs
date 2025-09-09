use crate::config::AuthConfig;

#[derive(Clone, Debug)]
pub struct ApiKeyAuth {
    enabled: bool,
    header_name: String,
    // For now keep raw secrets; replace with hashed+DB in production
    allowed: Vec<String>,
}

impl ApiKeyAuth {
    pub fn new(cfg: &AuthConfig) -> Self {
        Self {
            enabled: cfg.enabled,
            header_name: cfg.header_name.clone(),
            allowed: cfg.allowed_keys.clone(),
        }
    }

    pub fn header_name(&self) -> &str {
        &self.header_name
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn validate(&self, presented: Option<&str>) -> bool {
        if !self.enabled {
            return true; // auth disabled
        }
        let key = match presented {
            Some(k) if !k.is_empty() => k,
            _ => return false,
        };
        // Constant-time-ish equality check across allowed keys
        self.allowed
            .iter()
            .any(|allowed| constant_time_eq(allowed.as_bytes(), key.as_bytes()))
    }
}

// Minimal constant-time equality to avoid timing leaks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut r: u8 = 0;
    for i in 0..a.len() {
        r |= a[i] ^ b[i];
    }
    r == 0
}
