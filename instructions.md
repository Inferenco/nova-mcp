# Production-readiness report for nova‑mcp

This report analyses the nova‑mcp repository provided by the user and describes what is required to make it production‑ready. It includes a plan for gating access with API keys, hardening the HTTP surface, adding rate limiting and observability, and reorganising code to respect the user's module‑layout conventions. Citations from publicly available sources support each recommendation.

## Current state of the repository

Nova‑mcp is a minimal MCP (Model Context Protocol) server implemented in Rust. At present it only exposes two example tools (cat facts and BTC price). Key observations from the code and configuration include:

1. **Transport** — The server supports stdio by default and an optional HTTP JSON‑RPC interface at POST /rpc. There is no TLS termination, authentication or rate limiting on the HTTP endpoint.
2. **Core logic** — NovaServer in src/server.rs implements MCP methods (tools/list, tools/call, initialize, ping). Tool dispatch and DTOs are in the same file, breaking the user's modular‑design rules.
3. **Configuration** — NovaConfig is loaded from environment variables or TOML. It defines server.port, log_level, transport and API‑key values for third‑party APIs. There is a rate_limit_per_minute field, but no enforcement.
4. **HTTP server** — src/http.rs spins up an axum server with a single /rpc route. There are no layers for authentication, size limits or observability.
5. **Error handling** — The code uses custom error types and logging but relies on unwrap in a few places (e.g., parsing tool arguments), which could panic.

## Risks and missing controls

The repository works well for demos but lacks several requirements for production:

- **Unauthenticated HTTP** — Anyone can call /rpc. OWASP's API security project highlights broken authentication as a major risk for APIs[1]. We need to gate each request with an API key (and prepare for OAuth/JWT migration).
- **No rate limiting or quotas** — Without limits, a single caller can exhaust resources. OWASP notes that APIs should implement rate limiting and throttling to prevent abuse and denial of service[2].
- **Secrets management** — API keys are read directly from environment variables or config files. The OWASP secrets management cheat sheet warns that secrets should not be hard‑coded in Dockerfiles or exposed in environment variables; instead, orchestrators or vaults should inject them at runtime[3].
- **Lack of input validation** — Parameters passed to tools are deserialised without validation. Input validation guidelines recommend not trusting input parameters and to validate their length, range, format and type[4].
- **Observability gaps** — There are no metrics or traces; only basic logging. Production systems benefit from OpenTelemetry with smart sampling to handle large volumes of traces[5].
- **No health/readiness endpoints** — There is no way for orchestration systems to check liveness or readiness.
- **Single Sled instance missing** — The code doesn't currently use Sled (a simple key–value store), but to manage API keys and usage counters we propose adding it. The user's rules require opening Sled only once and passing around the Arc<Db>.

## Design goals for production

The design must satisfy the user's module‑structure rules (DTOs in dto.rs, methods in impl.rs, handlers in handler.rs, helpers in helpers.rs, root exports via mod.rs, no unwrap(), etc.) and provide the following features:

1. **Secure authentication** — Gate all HTTP requests with an API key in a header (x‑api‑key) and verify it against a store of hashed secrets. Optionally support HMAC signatures (x‑signature with x‑timestamp) to ensure body integrity and defend against replay attacks.
2. **Fine‑grained authorisation** — Map API keys to scopes corresponding to tool names. Only return tools in tools/list if the caller has the appropriate scope. Prepare the design to swap out API keys for OAuth 2.1 or JWT when required.
3. **Rate limiting & quotas** — Apply per‑IP and per‑key rate limits. Use token‑bucket algorithms (via tower-governor) for in‑memory enforcement and persist usage counters in Sled for audit/billing. OWASP recommends rate limiting to mitigate brute‑force attacks[2].
4. **Secrets management** — Remove secrets from the Dockerfile and environment variables; use orchestrator secret injection or a secret manager. OWASP's guidance notes that environment variables may be accessible to all processes and can leak in logs[3].
5. **Observability** — Integrate OpenTelemetry for traces and metrics. Implement RED metrics (request rate, error rate, duration) and propagate request IDs. Use sampling strategies to control the telemetry volume[5].
6. **HTTP hardening** — Serve JSON‑RPC via /rpc only; add health (/healthz) and readiness (/readyz) endpoints. Enforce request size limits (e.g., 1–2 MB), header size limits, timeouts and disallow other methods. Use TLS termination at the edge and set security headers such as X‑Content‑Type‑Options: nosniff. Only allow POST /rpc and GET for health checks[6].
7. **Module reorganisation** — Split the code into modules as per the user's rules: data‑only structs in dto.rs, business logic in impl.rs, handlers in handler.rs and local helpers in helpers.rs, with re‑exports in mod.rs. Remove all unwrap() except in tests or prototypes.
## Proposed module layout

```
src/
  auth/
    dto.rs        // ApiKeyRecord, Principal, SignatureHeaders
    impl.rs       // ApiKeyStore (Sled‑backed), HMAC verification
    helpers.rs    // request canonicaliser, constant‑time compares
    mod.rs

  ratelimit/
    dto.rs        // RateLimitConfig { per_key_per_min, per_ip_per_min, burst }
    impl.rs       // builds Tower‑governor layers and per‑key counters
    mod.rs

  mcp/
    dto.rs        // McpRequest, McpResponse, Tool, ToolCall, ToolResult
    impl.rs       // NovaServer: get_tools, handle_request, handle_tool_call
    handler.rs    // Axum handler for `/rpc`, auth + limits
    mod.rs

  config/
    dto.rs        // NovaConfig, ServerConfig, ApiConfig, CacheConfig, AuthConfig, RateLimitConfig
    impl.rs       // from_env(), from_file(), validation
    mod.rs

  observability/
    impl.rs       // OpenTelemetry setup, metrics/traces
    mod.rs

  db/
    impl.rs       // open_db_once() returning `Arc<sled::Db>`
    mod.rs

  http/
    impl.rs       // builds the Axum router with layers
    mod.rs

  tools/
    dto.rs        // inputs/outputs for public tools
    impl.rs       // PublicTools methods for external APIs
    mod.rs

  error.rs        // NovaError and helper methods
  lib.rs           // re‑exports
  main.rs          // configuration, DB init, router startup
```
## Detailed implementation recommendations

### 1. API key store (auth/impl.rs)

Use Sled to persist API keys. Each record (ApiKeyRecord) stores a key_id, Argon2id hash of the secret, list of scopes, created_at, optional not_before/not_after, and revoked_at. When verifying x‑api‑key, split the header into key_id and secret, look up the record by key_id, check the activation window and revocation status, and verify the secret using Argon2id. Return a Principal with the key ID and scopes. Use constant‑time comparison for secrets.

To defend against request tampering, optionally require a HMAC signature: compute HMAC‑SHA256(secret, canonical_request) where canonical_request is a stable serialization of the JSON‑RPC request. Compare this with x‑signature after Base64 decoding and enforce a timestamp skew limit with x‑timestamp.

This design supports rotation by storing multiple versions of a key (e.g., separate signing secrets), revocation via revoked_at, and future migration to JWT/OAuth. API keys are recommended for low‑risk use cases; more advanced systems can adopt OAuth 2.1 or JSON Web Tokens later[2].

### 2. Rate limiting (ratelimit/impl.rs)

Implement per‑IP throttling using tower-governor, configured from RateLimitConfig. For per‑key limits, maintain in‑memory token buckets keyed by key_id and persist a daily usage counter in Sled under usage/<key_id>/<yyyyMMddHH>. Reject requests with HTTP 429 when limits are exceeded. OWASP's best practices emphasise rate limiting and throttling to mitigate brute‑force attacks[2].

### 3. Scope enforcement

Define scopes of the form tools:get_cat_fact, tools:get_btc_price and so on. When handling tools/list, only include tools for which the principal has the corresponding scope. When handling tools/call, check that the requested tool is in the principal's scope list. A separate tools:list scope can be used to permit introspection.

### 4. Input validation

Add validation of tool parameters. For example, define maximum lengths for strings, numeric ranges, and enforce types (e.g., max_length must be a number). Use Serde with #[serde(deserialize_with = ...)] to validate at deserialization time. According to API security guidance, never trust input parameters; validate length, range, format and type and use strong types[4]. Reject unexpected or illegal content and set appropriate HTTP status (e.g., 400 Bad Request or JSON‑RPC error code −32602).

### 5. Observability and logging

Initialize OpenTelemetry via opentelemetry-otlp and export traces and metrics to your telemetry back‑end. Use TraceLayer from tower-http to record HTTP spans and propagate request IDs from an x‑request-id header. Define RED metrics (request rate, error rate, latency distribution) per tool. Implement sampling strategies (e.g., probabilistic sampling) to control trace volume; the Better Stack guide notes that as systems grow, sampling helps reduce telemetry costs while preserving insight[5]. Include hashed key_id and tool names as attributes to correlate calls with principals.

### 6. HTTP transport and security

Build the Axum router in http/impl.rs to include layers:

- Request body limit (tower-http::limit::RequestBodyLimitLayer) to cap JSON body size to a configurable number of bytes.
- TraceLayer for structured logging.
- GovernorLayer for per‑IP rate limiting.
- CORS — disable all origins by default unless browser usage is expected.
- Only accept POST /rpc; add GET /healthz and GET /readyz. Health returns 200 if the process is alive; readiness ensures Sled and other dependencies are reachable. Unexpected paths return 404.
- Error mapping — convert internal errors to JSON‑RPC errors with generic messages to avoid leaking details.

Host the server behind a reverse proxy (Caddy/NGINX/Cloudflare) that terminates TLS 1.3, enforces HSTS and request size caps, and forwards a request ID header. The API authentication article emphasises enforcing TLS for API communications and whitelisting trusted origins[2].

### 7. Secrets management and deployment

Remove secrets from the Dockerfile and avoid ENV statements. Use Kubernetes secrets, Docker Swarm secrets or a cloud‑based secret manager (AWS Secrets Manager, Azure Key Vault, HashiCorp Vault). The OWASP secrets management cheat sheet warns that environment variables are widely accessible and should not be relied upon for sensitive data[3]. For example, mount secrets into the container at runtime and read them in main.rs with std::fs or a secret manager client.

Support configuration via environment variables (NOVA_MCP_PORT, NOVA_MCP_TRANSPORT, etc.) but validate them and apply sensible defaults.

### 8. Code refactor steps

1. Create new modules as outlined above. Move DTOs into dto.rs, methods into impl.rs, handlers into handler.rs, and helpers into helpers.rs. Add mod.rs files to re‑export only what is necessary. Use fully qualified root imports (e.g., crate::auth::dto::ApiKeyRecord) to comply with the user's rules.
2. Remove all unwrap() calls except in tests. Convert deserialisation and missing fields into Result and map them to proper errors.
3. Add new dependencies to Cargo.toml for authentication (argon2, hmac, base64), rate limiting (tower-governor), Sled and OpenTelemetry. Remove unused dependencies and run cargo check.
4. Implement CLI/admin tool (optional) to mint API keys by generating a random secret, hashing it and writing a record to Sled. Display the raw secret once on creation.
5. Write integration tests to ensure: requests without keys are rejected; unknown or revoked keys fail; invalid signatures are detected; scope enforcement works; rate limits trigger; and valid requests succeed.
## Deployment checklist

- Use a multi‑stage Dockerfile producing a minimal, non‑root container (scratch or distroless) and run with --cap-drop all and read‑only filesystems.
- Deploy behind a reverse proxy with TLS termination and WAF capabilities. Set X‑Content‑Type‑Options: nosniff and other security headers.
- Implement CI/CD to build, test and lint the code; run cargo check and cargo audit. Use blue/green deployments or canaries to minimise risk.
- Monitor p99 latency, error rate and rate‑limit rejections; set SLOs and alert thresholds.

## Conclusion

The existing nova‑mcp code offers a good starting point for an MCP server but lacks authentication, rate limiting, observability and compliance with the user's module standards. By introducing API keys with scopes, rate limiting, Sled persistence, input validation and comprehensive observability, and by restructuring the code into DTO, implementation, handler and helper modules, the server will be ready for production use. The recommendations above are grounded in OWASP guidelines for API security and secrets management[1][2][3], as well as best practices for observability[5] and input validation[4].

---

## References

[1] [OWASP API Security Project | OWASP Foundation](https://owasp.org/www-project-api-security/)

[2] [API Authentication Security - Best Practices](https://www.impart.security/api-security-best-practices/api-authentication-security-best-practices)

[3] [Secrets Management - OWASP Cheat Sheet Series](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)

[4] [REST API Security Considerations](https://www.sandeepseeram.com/post/restful-apis-security-considerations)

[5] [Essential OpenTelemetry Best Practices for Robust Observability | Better Stack Community](https://betterstack.com/community/guides/observability/opentelemetry-best-practices/)
