---
title: "XRPC Proxy"
---

Control which unrecognized XRPC methods HappyView forwards to their resolved authority. Locally registered lexicons are always served regardless of this setting.

All endpoints require the `settings:manage` permission.

```ts tab="TypeScript" tab-group="language"
const TOKEN = "hv_..."; // your API key
const headers = { Authorization: `Bearer ${TOKEN}` };
```
```js tab="JavaScript" tab-group="language"
const TOKEN = "hv_..."; // your API key
const headers = { Authorization: `Bearer ${TOKEN}` };
```
```rust tab="Rust" tab-group="language"
let token = "hv_..."; // your API key
```
```go tab="Go" tab-group="language"
token := "hv_..." // your API key
```
```sh tab="cURL" tab-group="language"
# All examples assume $TOKEN is an API key (hv_...)
AUTH="Authorization: Bearer $TOKEN"
```

## Get proxy config

```
GET /admin/settings/xrpc-proxy
```

```ts tab="TypeScript" tab-group="language"
interface XrpcProxyConfig {
  mode: string;
  nsids: string[];
}

const response = await fetch("http://127.0.0.1:3000/admin/settings/xrpc-proxy", {
  headers,
});
const data: XrpcProxyConfig = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/admin/settings/xrpc-proxy", {
  headers,
});
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .get("http://127.0.0.1:3000/admin/settings/xrpc-proxy")
    .bearer_auth(token)
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
req, _ := http.NewRequest("GET", "http://127.0.0.1:3000/admin/settings/xrpc-proxy", nil)
req.Header.Set("Authorization", "Bearer "+token)
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl http://127.0.0.1:3000/admin/settings/xrpc-proxy -H "$AUTH"
```

**Response**: `200 OK`

```json
{
  "mode": "allowlist",
  "nsids": ["com.example.feed.*", "games.gamesgamesgamesgames.*"]
}
```

Returns `{"mode": "open", "nsids": []}` when no config has been saved.

## Update proxy config

```
PUT /admin/settings/xrpc-proxy
```

```ts tab="TypeScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/admin/settings/xrpc-proxy", {
  method: "PUT",
  headers: {
    ...headers,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    mode: "allowlist",
    nsids: ["com.example.feed.*"],
  }),
});
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/admin/settings/xrpc-proxy", {
  method: "PUT",
  headers: {
    ...headers,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    mode: "allowlist",
    nsids: ["com.example.feed.*"],
  }),
});
```
```rust tab="Rust" tab-group="language"
let response = client
    .put("http://127.0.0.1:3000/admin/settings/xrpc-proxy")
    .bearer_auth(token)
    .json(&serde_json::json!({
        "mode": "allowlist",
        "nsids": ["com.example.feed.*"]
    }))
    .send()
    .await?;
```
```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{
  "mode": "allowlist",
  "nsids": ["com.example.feed.*"]
}`)
req, _ := http.NewRequest("PUT", "http://127.0.0.1:3000/admin/settings/xrpc-proxy", body)
req.Header.Set("Authorization", "Bearer "+token)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X PUT http://127.0.0.1:3000/admin/settings/xrpc-proxy \
  -H "$AUTH" \
  -H "Content-Type: application/json" \
  -d '{
    "mode": "allowlist",
    "nsids": ["com.example.feed.*"]
  }'
```

**Response**: `204 No Content`

Changes take effect immediately — no restart needed.

### Modes

| Mode | Behavior |
|------|----------|
| `disabled` | Block all proxy requests. Return `403` for every unrecognized NSID. |
| `open` | Proxy everything (default). Current behavior on a fresh install. |
| `allowlist` | Proxy only NSIDs matching a pattern in `nsids`. Return `403` for the rest. |
| `blocklist` | Proxy everything except NSIDs matching a pattern in `nsids`. |

When mode is `disabled` or `open`, any `nsids` in the request body are ignored and stored as `[]`.

### NSID patterns

Patterns are dotted NSID identifiers. Trailing wildcards are supported:

- `com.example.feed.getHot` — exact match
- `com.example.feed.*` — matches any NSID starting with `com.example.feed.`
- `games.gamesgamesgamesgames.*` — matches the entire namespace

Mid-segment wildcards (e.g., `com.*.feed`) are not supported.

### Validation errors

| Status | Cause |
|--------|-------|
| `400` | An NSID pattern is empty, has fewer than two segments, contains invalid characters, or uses an unsupported wildcard |
| `422` | `mode` is not one of `disabled`, `open`, `allowlist`, `blocklist` |

## Blocked request response

When the proxy denies a request, the client receives:

```
403 Forbidden
```

```json
{
  "error": "NSID not allowed by proxy policy"
}
```
