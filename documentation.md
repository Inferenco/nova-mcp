# Nova-MCP Documentation

## Overview

- Purpose: Minimal MCP server exposing GeckoTerminal-powered DEX tools via JSON-RPC for OpenAI Responses’ MCP tool.
- Transports: `stdio` (default) and `http` JSON-RPC.
- Auth: Optional API key required for HTTP, disabled for stdio.
- Extras: Simple plugin registry with enablement per user/group (dev/demo), sled-backed state, basic rate limiting.

## Architecture

```
src/
├── main.rs                 # Entrypoint; selects transport (stdio/http)
├── server.rs               # Server object; tool registry; PluginManager wiring
├── mcp/
│   ├── dto.rs              # JSON-RPC types for MCP
│   └── handler.rs          # Implements initialize, tools/list, tools/call, ping
├── http.rs                 # HTTP transport (/rpc + /plugins/* + health)
├── auth.rs                 # API key header validation
├── config.rs               # Env/TOML-driven config (serde defaulted)
├── plugins/
│   ├── dto.rs              # Plugin metadata + enablement records
│   ├── handler.rs          # REST handlers (register/update/list/invoke/enable)
│   ├── helpers.rs          # Auth + rate limiting integration for plugins
│   └── manager.rs          # In-memory registry + sled-backed enablement
└── tools/
    ├── mod.rs              # Public re-exports for tools
    └── gecko_terminal/
        ├── helpers.rs
        ├── implementation.rs   # Shared reqwest client + base URL
        ├── networks/           # get_gecko_networks
        │   ├── dto.rs
        │   └── handler.rs
        ├── token/              # get_gecko_token
        │   ├── dto.rs
        │   └── handler.rs
        ├── pool/               # get_gecko_pool
        │   ├── dto.rs
        │   └── handler.rs
        ├── trending_pools/     # get_trending_pools
        │   ├── dto.rs
        │   ├── handler.rs
        │   └── implementation.rs
        ├── search_pools/       # search_pools
        │   ├── dto.rs
        │   ├── handler.rs
        │   └── implementation.rs
        └── new_pools/          # get_new_pools
            ├── dto.rs
            ├── handler.rs
            └── implementation.rs
```

## Tools

- get_gecko_networks: Lists available networks.
- get_gecko_token: Returns token info on a network/address.
- get_gecko_pool: Returns pool info on a network/address.
- get_trending_pools: Lists trending pools with pagination and duration.
- search_pools: Searches pools by query, optional network.
- get_new_pools: Lists newest pools with pagination.

Schemas are defined in `src/server.rs:get_tools()` and inputs/outputs live in the module `dto.rs` files.

## MCP JSON-RPC

- initialize: Returns protocol version and server info.
- tools/list: Returns tools with name/description/input_schema.
- tools/call: Executes the tool by name and `arguments` object.

Example request/response for tools/list:

```
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
```

```
{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"get_gecko_networks", ...}]}}
```

## HTTP Transport

- Endpoint: `POST /rpc` with JSON body as `McpRequest`.
- Auth: Header name defaults to `x-api-key` when enabled. Configure header/key(s) via env.
- Health: `GET /healthz` and `GET /readyz`.
- Rate limit: Simple per-key counter with a minute bucket and TTL cleanup.

## Plugin Registry (Dev)

- Register: `POST /plugins/register` -> `PluginMetadata`.
- Update: `PUT /plugins/:plugin_id` -> `PluginMetadata`.
- Unregister: `DELETE /plugins/:plugin_id`.
- List: `GET /plugins` -> `PluginMetadata[]`.
- Enablement: `POST /plugins/enable` -> `PluginEnablementStatus` for user or group.
- Invoke: `POST /plugins/:plugin_id/call` with context and arguments.

Enablement is stored in sled (`user_plugins`, `group_plugins` trees). This is a demonstration scaffold; swap out for your production policy store.

## Configuration

Environment variables:

```
# Server
NOVA_MCP_TRANSPORT=stdio|http
NOVA_MCP_PORT=8080
NOVA_MCP_LOG_LEVEL=info

# HTTP auth
NOVA_MCP_AUTH_ENABLED=true|false
NOVA_MCP_API_KEYS="key1,key2"
NOVA_MCP_AUTH_HEADER=x-api-key

# External APIs
GECKO_TERMINAL_BASE_URL=https://api.geckoterminal.com/api/v2
UNISWAP_API_KEY=...
COINGECKO_API_KEY=...
DEXSCREENER_API_KEY=...

# Limits/cache
# (rate_limit_per_minute, ttl_seconds, etc., when using config file)
```

TOML config (`config.toml`) mirrors `NovaConfig` (see README for example). You can load from file using `NovaConfig::from_file(path)` or rely on `from_env()`.

## Running

- Stdio: `cargo run --bin nova-mcp-stdio`
- HTTP: set `NOVA_MCP_TRANSPORT=http` and `NOVA_MCP_PORT`, then run the same command.
- Docker: see README for full Compose and CLI examples.

## Testing

- Unit/integration: `cargo test`
- Live API tests (ignored): `cargo test -- --ignored`

## Adding a Tool

1. Create a `your_tool/` directory under `src/tools/gecko_terminal/` with `dto.rs`, `handler.rs`, and optional `implementation.rs`.
2. Re-export it in `src/tools/gecko_terminal/mod.rs` (and in `src/tools/mod.rs` if you want top-level re-exports).
3. Register the tool schema in `src/server.rs:get_tools()`.
4. Add a `match` branch in `src/mcp/handler.rs:handle_tool_call()` for the tool name.
5. Add tests under `tests/` and, optionally, live tests under `tests/` with `#[ignore]`.

## Error Handling

- Internal errors are surfaced as `McpError` with code `-32603` in JSON-RPC and appropriate HTTP codes in the HTTP transport and plugin routes.
- Common validation errors return concise messages (e.g., missing required params).

## Security Notes

- HTTP auth uses raw API keys for demo; consider a proper identity layer with hashed secrets and scoped tokens in production.
- Rate limiting is in-memory per-process; use a shared limiter (Redis) for multi-instance deployments.
- Sled storage is local; replace with a managed DB for production needs.

## Troubleshooting

- “Unauthorized” on HTTP: Make sure `NOVA_MCP_AUTH_ENABLED=true` and include the header (`x-api-key: your_key`).
- Rate limit exceeded: Increase `rate_limit_per_minute` or reduce request frequency.

## Nova-MCP Upgrade Plan – User & Group Context

### 1. Introduction and Scope

Nova-MCP currently exposes a fixed set of GeckoTerminal tools through the Model Context Protocol (MCP) and offers a basic plugin registry that stores HTTP endpoints. This upgrade introduces multi-tenant tooling by associating every tool with a **context**—either an individual user or a Telegram group. Requests are still authenticated with a single API key, while the caller identity is derived from the `(context_type, context_id)` pair supplied with each request. The plan below supersedes earlier drafts by clarifying context handling and simplifying authentication.

### 2. Current Architecture Summary

- Built-in tools are defined by `Tool { name, description, input_schema }` structs and assembled in `NovaServer::get_tools()`.
- Plugins are registered with `/plugins/register` via `PluginRegistrationRequest` (fields: `name`, `description`, `owner_id`, `scopes`, `endpoint`, `trust_level`).
- Plugins can be enabled per user or per group through `PluginEnableRequest`, which carries `context_type` (`User` or `Group`) and `context_id`.
- Authentication uses a single API key configured via environment variables to guard the HTTP API.

### 3. Revised Requirements

1. **User and Group Context** – Tools are owned by `(context_type, context_id)` pairs. The values are stored with each tool and used for filtering.
2. **Single API Key** – All HTTP requests share one API key. Each request must also include the caller context headers.
3. **Web Dashboard** – Provide a browser UI for defining schemas, registering tools, managing enablement, and generating implementation stubs per context.
4. **Backward Compatibility** – Built-in tools and the existing plugin registry continue to function without breaking current clients.
5. **Security and Isolation** – Tools run as external services. Nova validates inputs against declared schemas and ensures enablement matches the request context.

### 4. Data Model Changes

#### 4.1 Tool Ownership

Extend `PluginRegistrationRequest` (and related structs) with:

| Field          | Type                     | Description                                                                 |
| -------------- | ------------------------ | --------------------------------------------------------------------------- |
| `context_type` | `PluginContextType`      | Either `User` or `Group`; identifies personal vs. group ownership.          |
| `context_id`   | `String`                 | Telegram user ID or group ID used to scope the tool.                        |
| `input_schema` | `serde_json::Value`      | JSON schema for tool arguments.                                             |
| `output_schema`| `Option<serde_json::Value>` | Optional JSON schema describing the result payload.                      |
| `version`      | `u32`                    | Tool definition version number.                                             |

During registration the server validates that `context_type` and `context_id` match Telegram semantics (e.g., group IDs are negative). The existing `owner_id` remains available for internal references.

#### 4.2 Tool Namespacing

Fully qualified tool names prevent collisions:

```rust
if context_type == User {
    fq_name = format!("user_{}_{}_v{}", context_id, name, version);
} else {
    fq_name = format!("group_{}_{}_v{}", context_id, name, version);
}
```

The `tools/list` response returns this fully qualified name. Descriptions continue to display friendly names and ownership details.

### 5. API and Protocol Changes

#### 5.1 Context Identification

- **HTTP headers** – Require `x-api-key`, `x-nova-context-type`, and `x-nova-context-id` on every call.
- **JSON-RPC (stdio)** – Add optional root-level `context_type` and `context_id` fields; reject user-defined tool calls when missing.
- **Middleware** – Extract context values and pass them to listing/invocation logic. Invalid or missing values return authentication errors.

#### 5.2 Tool Registration

Introduce `/tools/register` that:

1. Validates request headers against body context, checks schema shapes, and prevents intra-context name conflicts.
2. Persists metadata (`PluginMetadata`) including schemas, computes the fully qualified name, and assigns `plugin_id`.
3. Automatically enables the tool for its owning context.
4. Returns the `plugin_id`, fully qualified name, and version.

`PUT /tools/:plugin_id` follows the same rules, incrementing `version`. Older versions remain accessible for compatibility.

#### 5.3 Tool Listing

`tools/list` now:

1. Extracts the caller context.
2. Loads built-in tools as before.
3. Queries the plugin manager for enabled plugins matching the caller context.
4. Converts each plugin to a `Tool` using the fully qualified name and stored schemas.
5. Returns the combined list.

#### 5.4 Tool Invocation

When `tools/call` is invoked:

1. Dispatch built-in tools normally when names match.
2. Otherwise parse the fully qualified name into `(context_type, context_id, base_name, version)`.
3. Verify the caller context matches the parsed context.
4. Confirm the tool is enabled for the caller.
5. Validate arguments against `input_schema`.
6. Invoke the plugin endpoint with `PluginInvocationRequest { context_type, context_id, arguments }` and validate responses against `output_schema` (when provided).

#### 5.5 Other Plugin Endpoints

Keep existing `/plugins/*` endpoints for admin workflows. `/tools/*` becomes the primary user surface.

### 6. Server Implementation Changes

#### 6.1 Auth Middleware

- Parse `x-nova-context-type` and `x-nova-context-id` alongside `x-api-key`.
- Validate type/value combinations (e.g., numeric IDs, negative group IDs) and store them in request context.

#### 6.2 Plugin Manager

- Extend `PluginMetadata` with context, schema, and version fields.
- Add helpers such as `list_plugins_for_context` and `get_plugin_by_fq_name`.
- Support versioning by storing multiple versions per `(context_type, context_id, base_name)` and defaulting to the highest.
- Automatically record enablement for the owner context at registration time.

#### 6.3 MCP Handlers

- Update `NovaServer::get_tools()` to accept context, merge built-in tools with context-filtered plugins, and return the aggregated list.
- Update `handle_tool_call()` to parse fully qualified names, enforce context checks, and route invocations through the plugin manager.

### 7. Web Dashboard

#### 7.1 Authentication

- Prompt for API key and user/group ID, store them in session memory, and include them in every request.
- Display the active context and allow quick switching between personal and group scopes.

#### 7.2 Tool Creation Workflow

1. Select context (auto-detect groups via negative IDs).
2. Define tool metadata (name, description) with per-context uniqueness.
3. Build input schema using guided helpers and inline validation.
4. (Optional) Define an output schema.
5. Provide implementation details (HTTPS endpoint or generated stub) with expectations for JSON POST payloads/responses.
6. Register via `/tools/register` and surface the fully qualified tool name.
7. Optionally enable the tool for additional contexts using `/plugins/enable`.

#### 7.3 Manage Tools

- List tools for the current context with base name, version, status, and creation date.
- Support enabling/disabling, editing (with version bumps), deletion, and cross-context enablement.
- (Optional) Display invocation logs for diagnostics.

#### 7.4 Implementation Notes

- Build as a React SPA served by Nova or standalone with configurable API base URL.
- Use validated forms (e.g., React Hook Form) and provide a JSON editor for advanced schema editing.
- Editing a tool clones metadata, increments the version, and publishes the new definition as the default in `tools/list`.

### 8. Security and Governance

- Implement per-context rate limiting (registrations and invocations).
- Sanitize schemas to allow only safe JSON Schema keywords.
- Enforce ownership rules—only the owning context can update/delete; group admins may receive elevated rights via existing management features.
- Record audit logs with actor, timestamp, context, and IP metadata. Provide an admin route to inspect logs.
- Keep the design extensible for future context types.

### 9. Testing

- Unit tests: context parsing, name formatting, plugin filtering.
- Integration tests: registration/listing/invocation flows, context mismatch handling.
- UI tests: dashboard context selection and management workflows.

### 10. Roll-out Plan

1. **Design Review** – Validate context model and Telegram ID compatibility.
2. **Back-End Implementation (~3 weeks)** – Update plugin manager, auth layer, tool registration, and MCP handlers; add endpoints and versioning.
3. **Dashboard Implementation (~3–4 weeks)** – Build/update UI for context workflows.
4. **Testing & QA (2 weeks)** – Verify context isolation and regression coverage for built-in tools.
5. **Documentation & Communication (1 week)** – Update README/docs and provide usage examples.
6. **Deployment** – Stage, migrate plugin data to include context fields, release to production, and monitor logs for misconfigured contexts.

### 11. Conclusion

By anchoring every tool to a `(context_type, context_id)` pair, Nova-MCP becomes a multi-tenant platform while retaining the simplicity of a single API key. The web dashboard empowers individuals and groups to build, test, and manage their tools, and the back end enforces validation, versioning, and context isolation for a safe ecosystem.
- Live API flakiness: GeckoTerminal is public; expect occasional rate limits or transient errors; retry or add caching if needed.

