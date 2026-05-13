---
title: "Update or Delete"
---

A single endpoint that handles create, update, and delete based on the input fields.

**Lexicon type:** procedure

```lua
function handle()
  if input.delete and input.uri then
    local r = Record.load(input.uri)
    if r then r:delete() end
    return { success = true }
  end

  if input.uri then
    -- Update existing
    local r = Record.load(input.uri)
    if not r then error("not found") end
    r.status = input.status
    r:save()
    return { uri = r._uri, cid = r._cid }
  end

  -- Create new
  local r = Record(collection, input)
  r:save()
  return { uri = r._uri, cid = r._cid }
end
```

## How it works

1. If `input.delete` is truthy and `input.uri` is provided, load the record with [`Record.load`](../../api-reference/lua/record-api.md#static-methods) and delete it.
2. If only `input.uri` is provided, load the existing record with [`Record.load`](../../api-reference/lua/record-api.md#static-methods), update its fields, and save it back. Since `_uri` is already set, `r:save()` calls `putRecord` instead of `createRecord`.
3. If neither condition matches, create a new record from the input.

## Usage

**Create:**

```ts tab="TypeScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    Authorization: `Bearer ${TOKEN}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({ status: "hello" }),
});
const data = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    Authorization: `Bearer ${TOKEN}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({ status: "hello" }),
});
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord")
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("Bearer {}", token))
    .json(&serde_json::json!({ "status": "hello" }))
    .send()
    .await?;
```
```go tab="Go" tab-group="language"
body := `{ "status": "hello" }`
req, _ := http.NewRequest("POST", "http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", bytes.NewBufferString(body))
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "Bearer "+token)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord \
  -H "X-Client-Key: $CLIENT_KEY" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{ "status": "hello" }'
```

**Update:**

```ts tab="TypeScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    Authorization: `Bearer ${TOKEN}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    uri: "at://did:plc:abc/xyz.statusphere.record/abc123",
    status: "updated",
  }),
});
const data = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    Authorization: `Bearer ${TOKEN}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    uri: "at://did:plc:abc/xyz.statusphere.record/abc123",
    status: "updated",
  }),
});
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord")
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("Bearer {}", token))
    .json(&serde_json::json!({
        "uri": "at://did:plc:abc/xyz.statusphere.record/abc123",
        "status": "updated"
    }))
    .send()
    .await?;
```
```go tab="Go" tab-group="language"
body := `{ "uri": "at://did:plc:abc/xyz.statusphere.record/abc123", "status": "updated" }`
req, _ := http.NewRequest("POST", "http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", bytes.NewBufferString(body))
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "Bearer "+token)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord \
  -H "X-Client-Key: $CLIENT_KEY" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{ "uri": "at://did:plc:abc/xyz.statusphere.record/abc123", "status": "updated" }'
```

**Delete:**

```ts tab="TypeScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    Authorization: `Bearer ${TOKEN}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    uri: "at://did:plc:abc/xyz.statusphere.record/abc123",
    delete: true,
  }),
});
const data = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    Authorization: `Bearer ${TOKEN}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    uri: "at://did:plc:abc/xyz.statusphere.record/abc123",
    delete: true,
  }),
});
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord")
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("Bearer {}", token))
    .json(&serde_json::json!({
        "uri": "at://did:plc:abc/xyz.statusphere.record/abc123",
        "delete": true
    }))
    .send()
    .await?;
```
```go tab="Go" tab-group="language"
body := `{ "uri": "at://did:plc:abc/xyz.statusphere.record/abc123", "delete": true }`
req, _ := http.NewRequest("POST", "http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord", bytes.NewBufferString(body))
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "Bearer "+token)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST http://127.0.0.1:3000/xrpc/xyz.statusphere.setRecord \
  -H "X-Client-Key: $CLIENT_KEY" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{ "uri": "at://did:plc:abc/xyz.statusphere.record/abc123", "delete": true }'
```

## Use case

This pattern reduces the number of endpoints your app needs by multiplexing create, update, and delete through a single procedure. The presence of `uri` and `delete` fields in the input determines the action.
