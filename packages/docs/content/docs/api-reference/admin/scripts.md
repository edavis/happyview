---
title: "Scripts"
---

Manage trigger-keyed scripts. Scripts run automatically in response to events like record indexing, XRPC calls, or labeler actions. The trigger id (e.g. `record.index:xyz.statusphere.status`) determines when a script fires.

**Permissions:** `scripts:read` for GET endpoints, `scripts:manage` for mutating endpoints.

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

## List scripts

```
GET /admin/scripts
```

Optionally filter by NSID suffix with the `?suffix=` query parameter.

```ts tab="TypeScript" tab-group="language"
interface Script {
  id: string;
  script_type: string;
  body: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

// List all scripts
const response = await fetch("http://127.0.0.1:3000/admin/scripts", {
  headers,
});
const data: Script[] = await response.json();

// Filter by NSID suffix
const filtered = await fetch(
  "http://127.0.0.1:3000/admin/scripts?suffix=xyz.statusphere.status",
  { headers },
);
const filteredData: Script[] = await filtered.json();
```
```js tab="JavaScript" tab-group="language"
// List all scripts
const response = await fetch("http://127.0.0.1:3000/admin/scripts", {
  headers,
});
const data = await response.json();

// Filter by NSID suffix
const filtered = await fetch(
  "http://127.0.0.1:3000/admin/scripts?suffix=xyz.statusphere.status",
  { headers },
);
const filteredData = await filtered.json();
```
```rust tab="Rust" tab-group="language"
// List all scripts
let response = client
    .get("http://127.0.0.1:3000/admin/scripts")
    .bearer_auth(token)
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;

// Filter by NSID suffix
let response = client
    .get("http://127.0.0.1:3000/admin/scripts?suffix=xyz.statusphere.status")
    .bearer_auth(token)
    .send()
    .await?;
let filtered: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
// List all scripts
req, _ := http.NewRequest("GET", "http://127.0.0.1:3000/admin/scripts", nil)
req.Header.Set("Authorization", "Bearer "+token)
resp, err := http.DefaultClient.Do(req)

// Filter by NSID suffix
req, _ = http.NewRequest("GET", "http://127.0.0.1:3000/admin/scripts?suffix=xyz.statusphere.status", nil)
req.Header.Set("Authorization", "Bearer "+token)
resp, err = http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
# List all scripts
curl http://127.0.0.1:3000/admin/scripts -H "$AUTH"

# Filter by NSID suffix
curl "http://127.0.0.1:3000/admin/scripts?suffix=xyz.statusphere.status" -H "$AUTH"
```

| Parameter | Type   | Required | Description                                                      |
| --------- | ------ | -------- | ---------------------------------------------------------------- |
| `suffix`  | string | no       | Filter to scripts whose id ends with `:<suffix>` (query param)   |

**Response**: `200 OK`

```json
[
  {
    "id": "record.index:xyz.statusphere.status",
    "script_type": "lua",
    "body": "function handle()\n  return event\nend",
    "description": "Process indexed statuses",
    "created_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-01T00:00:00Z"
  }
]
```

## Get a script

```
GET /admin/scripts/{id}
```

The `{id}` path parameter is the trigger string, URL-encoded (e.g. `record.index%3Axyz.statusphere.status`).

```ts tab="TypeScript" tab-group="language"
const response = await fetch(
  "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status",
  { headers },
);
const data: Script = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch(
  "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status",
  { headers },
);
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .get("http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status")
    .bearer_auth(token)
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
req, _ := http.NewRequest("GET", "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status", nil)
req.Header.Set("Authorization", "Bearer "+token)
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status" -H "$AUTH"
```

**Response**: `200 OK`

```json
{
  "id": "record.index:xyz.statusphere.status",
  "script_type": "lua",
  "body": "function handle()\n  return event\nend",
  "description": "Process indexed statuses",
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

## Create or replace a script

```
POST /admin/scripts
```

Creates a new script or replaces an existing one by `id`. The trigger grammar and Lua body are validated at write-time.

```ts tab="TypeScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/admin/scripts", {
  method: "POST",
  headers: {
    ...headers,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    id: "record.index:xyz.statusphere.status",
    script_type: "lua",
    body: "function handle()\n  return event\nend",
    description: "Process indexed statuses",
  }),
});
const data: Script = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/admin/scripts", {
  method: "POST",
  headers: {
    ...headers,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    id: "record.index:xyz.statusphere.status",
    script_type: "lua",
    body: "function handle()\n  return event\nend",
    description: "Process indexed statuses",
  }),
});
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("http://127.0.0.1:3000/admin/scripts")
    .bearer_auth(token)
    .json(&serde_json::json!({
        "id": "record.index:xyz.statusphere.status",
        "script_type": "lua",
        "body": "function handle()\n  return event\nend",
        "description": "Process indexed statuses"
    }))
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{
  "id": "record.index:xyz.statusphere.status",
  "script_type": "lua",
  "body": "function handle()\n  return event\nend",
  "description": "Process indexed statuses"
}`)
req, _ := http.NewRequest("POST", "http://127.0.0.1:3000/admin/scripts", body)
req.Header.Set("Authorization", "Bearer "+token)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST http://127.0.0.1:3000/admin/scripts \
  -H "$AUTH" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "record.index:xyz.statusphere.status",
    "script_type": "lua",
    "body": "function handle()\n  return event\nend",
    "description": "Process indexed statuses"
  }'
```

| Field         | Type   | Required | Description                                                    |
| ------------- | ------ | -------- | -------------------------------------------------------------- |
| `id`          | string | yes      | Trigger string (e.g. `record.index:xyz.statusphere.status`)    |
| `script_type` | string | no       | Script language; defaults to `"lua"`                           |
| `body`        | string | yes      | The script source code                                         |
| `description` | string | no       | Human-readable description (max 300 characters)                |

**Response**: `201 Created` (new) or `200 OK` (update)

```json
{
  "id": "record.index:xyz.statusphere.status",
  "script_type": "lua",
  "body": "function handle()\n  return event\nend",
  "description": "Process indexed statuses",
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

## Partial update a script

```
PATCH /admin/scripts/{id}
```

Updates individual fields of an existing script. At least one field must be provided. Setting `description` to `null` in JSON clears it. If `script_type` is changed, `body` must also be provided so validation can run against the new type.

```ts tab="TypeScript" tab-group="language"
const response = await fetch(
  "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status",
  {
    method: "PATCH",
    headers: {
      ...headers,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      description: "Updated description for status processing",
    }),
  },
);
const data: Script = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch(
  "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status",
  {
    method: "PATCH",
    headers: {
      ...headers,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      description: "Updated description for status processing",
    }),
  },
);
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .patch("http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status")
    .bearer_auth(token)
    .json(&serde_json::json!({
        "description": "Updated description for status processing"
    }))
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{
  "description": "Updated description for status processing"
}`)
req, _ := http.NewRequest("PATCH", "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status", body)
req.Header.Set("Authorization", "Bearer "+token)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X PATCH "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status" \
  -H "$AUTH" \
  -H "Content-Type: application/json" \
  -d '{ "description": "Updated description for status processing" }'
```

| Field         | Type         | Required | Description                                                      |
| ------------- | ------------ | -------- | ---------------------------------------------------------------- |
| `script_type` | string       | no       | Script language; requires `body` alongside                       |
| `body`        | string       | no       | New script source; re-validated against `script_type`            |
| `description` | string\|null | no       | New description, or `null` to clear                              |

**Response**: `200 OK`

```json
{
  "id": "record.index:xyz.statusphere.status",
  "script_type": "lua",
  "body": "function handle()\n  return event\nend",
  "description": "Updated description for status processing",
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

## Delete a script

```
DELETE /admin/scripts/{id}
```

```ts tab="TypeScript" tab-group="language"
const response = await fetch(
  "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status",
  {
    method: "DELETE",
    headers,
  },
);
```
```js tab="JavaScript" tab-group="language"
const response = await fetch(
  "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status",
  {
    method: "DELETE",
    headers,
  },
);
```
```rust tab="Rust" tab-group="language"
let response = client
    .delete("http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status")
    .bearer_auth(token)
    .send()
    .await?;
```
```go tab="Go" tab-group="language"
req, _ := http.NewRequest("DELETE", "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status", nil)
req.Header.Set("Authorization", "Bearer "+token)
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X DELETE "http://127.0.0.1:3000/admin/scripts/record.index%3Axyz.statusphere.status" \
  -H "$AUTH"
```

**Response**: `204 No Content`
