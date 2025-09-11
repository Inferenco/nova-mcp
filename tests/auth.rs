use nova_mcp::{config::AuthConfig, ApiKeyAuth};

#[test]
fn disabled_auth_allows_any() {
    let cfg = AuthConfig {
        enabled: false,
        allowed_keys: vec!["a".into()],
        header_name: "x".into(),
    };
    let auth = ApiKeyAuth::new(&cfg);
    assert!(auth.validate(None));
    assert!(auth.validate(Some("whatever")));
}

#[test]
fn enabled_auth_checks_keys() {
    let cfg = AuthConfig {
        enabled: true,
        allowed_keys: vec!["secret".into()],
        header_name: "x".into(),
    };
    let auth = ApiKeyAuth::new(&cfg);
    assert!(auth.validate(Some("secret")));
    assert!(!auth.validate(Some("wrong")));
    assert!(!auth.validate(None));
}
