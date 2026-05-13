---
title: "API Keys"
---

Manage API keys for programmatic access. See the [API Keys guide](../../guides/api-keys.md) for usage details.

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

## Create an API key

```
POST /admin/api-keys
```

Requires `api-keys:create` permission.

```ts tab="TypeScript" tab-group="language"
interface ApiKey {
  id: string;
  name: string;
  key: string;
  key_prefix: string;
  permissions: string[];
}

const response = await fetch("http://127.0.0.1:3000/admin/api-keys", {
  method: "POST",
  headers: {
    ...headers,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    name: "CI Deploy",
    permissions: ["lexicons:read", "lexicons:create", "backfill:create"],
  }),
});
const data: ApiKey = await response.json();
```

```js tab="JavaScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/admin/api-keys", {
  method: "POST",
  headers: {
    ...headers,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    name: "CI Deploy",
    permissions: ["lexicons:read", "lexicons:create", "backfill:create"],
  }),
});
const data = await response.json();
```

```rust tab="Rust" tab-group="language"
let client = reqwest::Client::new();
let response = client
    .post("http://127.0.0.1:3000/admin/api-keys")
    .bearer_auth(token)
    .json(&serde_json::json!({
        "name": "CI Deploy",
        "permissions": ["lexicons:read", "lexicons:create", "backfill:create"]
    }))
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```

```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{
  "name": "CI Deploy",
  "permissions": ["lexicons:read", "lexicons:create", "backfill:create"]
}`)
req, _ := http.NewRequest("POST", "http://127.0.0.1:3000/admin/api-keys", body)
req.Header.Set("Authorization", "Bearer "+token)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```

```sh tab="cURL" tab-group="language"
curl -X POST http://127.0.0.1:3000/admin/api-keys \
  -H "$AUTH" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "CI Deploy",
    "permissions": ["lexicons:read", "lexicons:create", "backfill:create"]
  }'
```

| Field         | Type     | Required | Description                                                                           |
| ------------- | -------- | -------- | ------------------------------------------------------------------------------------- |
| `name`        | string   | yes      | A label to identify this key's usage                                                  |
| `permissions` | string[] | yes      | Permissions to grant the key (must be a subset of the creating user's own permissions) |

**Response**: `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "CI Deploy",
  "key": "hv_a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4",
  "key_prefix": "hv_a1b2c3d4",
  "permissions": ["lexicons:read", "lexicons:create", "backfill:create"]
}
```

The `key` field contains the full API key. It is only returned in this response — store it securely. The key's effective permissions are the **intersection** of the permissions specified here and the creating user's permissions at the time of each request.

## List API keys

```
GET /admin/api-keys
```

Requires `api-keys:read` permission.

```ts tab="TypeScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/admin/api-keys", {
  headers,
});
const data: ApiKey[] = await response.json();
```

```js tab="JavaScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/admin/api-keys", {
  headers,
});
const data = await response.json();
```

```rust tab="Rust" tab-group="language"
let response = client
    .get("http://127.0.0.1:3000/admin/api-keys")
    .bearer_auth(token)
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```

```go tab="Go" tab-group="language"
req, _ := http.NewRequest("GET", "http://127.0.0.1:3000/admin/api-keys", nil)
req.Header.Set("Authorization", "Bearer "+token)
resp, err := http.DefaultClient.Do(req)
```

```sh tab="cURL" tab-group="language"
curl http://127.0.0.1:3000/admin/api-keys -H "$AUTH"
```

**Response**: `200 OK`

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "CI Deploy",
    "key_prefix": "hv_a1b2c3d4",
    "permissions": ["lexicons:read", "lexicons:create", "backfill:create"],
    "created_at": "2026-03-01T00:00:00Z",
    "last_used_at": "2026-03-06T12:00:00Z",
    "revoked_at": null
  }
]
```

Only returns keys belonging to the authenticated user. The full key is never included — only the prefix.

## Revoke an API key

```
DELETE /admin/api-keys/{id}
```

Requires `api-keys:delete` permission.

```ts tab="TypeScript" tab-group="language"
const response = await fetch(
  "http://127.0.0.1:3000/admin/api-keys/550e8400-e29b-41d4-a716-446655440000",
  { method: "DELETE", headers },
);
```

```js tab="JavaScript" tab-group="language"
const response = await fetch(
  "http://127.0.0.1:3000/admin/api-keys/550e8400-e29b-41d4-a716-446655440000",
  { method: "DELETE", headers },
);
```

```rust tab="Rust" tab-group="language"
client
    .delete("http://127.0.0.1:3000/admin/api-keys/550e8400-e29b-41d4-a716-446655440000")
    .bearer_auth(token)
    .send()
    .await?;
```

```go tab="Go" tab-group="language"
req, _ := http.NewRequest("DELETE",
  "http://127.0.0.1:3000/admin/api-keys/550e8400-e29b-41d4-a716-446655440000", nil)
req.Header.Set("Authorization", "Bearer "+token)
resp, err := http.DefaultClient.Do(req)
```

```sh tab="cURL" tab-group="language"
curl -X DELETE http://127.0.0.1:3000/admin/api-keys/550e8400-e29b-41d4-a716-446655440000 \
  -H "$AUTH"
```

Sets `revoked_at` on the key. The key remains in the database for audit purposes but can no longer authenticate.

**Response**: `204 No Content`
