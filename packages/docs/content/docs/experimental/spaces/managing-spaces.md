---
title: "Managing Spaces"
---

<Callout type="error" title="Experimental">
This API is experimental and will change. See the [Permissioned Spaces overview](../spaces.md) for context.
</Callout>

## Creating a space

```ts tab="TypeScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.createSpace", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    "Authorization": `DPoP ${ACCESS_TOKEN}`,
    "DPoP": DPOP_PROOF,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    type: "com.example.forum",
    skey: "main",
    displayName: "My Forum",
    description: "A place for discussion",
    accessMode: "default_allow",
  }),
});
interface CreateSpaceResponse {
  uri: string;
}
const data: CreateSpaceResponse = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.createSpace", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    "Authorization": `DPoP ${ACCESS_TOKEN}`,
    "DPoP": DPOP_PROOF,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    type: "com.example.forum",
    skey: "main",
    displayName: "My Forum",
    description: "A place for discussion",
    accessMode: "default_allow",
  }),
});
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("https://happyview.example.com/xrpc/dev.happyview.space.createSpace")
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("DPoP {}", access_token))
    .header("DPoP", &dpop_proof)
    .json(&serde_json::json!({
        "type": "com.example.forum",
        "skey": "main",
        "displayName": "My Forum",
        "description": "A place for discussion",
        "accessMode": "default_allow"
    }))
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{
  "type": "com.example.forum",
  "skey": "main",
  "displayName": "My Forum",
  "description": "A place for discussion",
  "accessMode": "default_allow"
}`)
req, _ := http.NewRequest("POST",
  "https://happyview.example.com/xrpc/dev.happyview.space.createSpace", body)
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "DPoP "+accessToken)
req.Header.Set("DPoP", dpopProof)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST 'https://happyview.example.com/xrpc/dev.happyview.space.createSpace' \
  -H 'X-Client-Key: hvc_...' \
  -H 'Authorization: DPoP <token>' \
  -H 'DPoP: <proof>' \
  -H 'Content-Type: application/json' \
  -d '{
    "type": "com.example.forum",
    "skey": "main",
    "displayName": "My Forum",
    "description": "A place for discussion",
    "accessMode": "default_allow"
  }'
```

**Input:**

| Field            | Type          | Required | Description                                       |
| ---------------- | ------------- | -------- | ------------------------------------------------- |
| `type`           | string (NSID) | Yes      | The space type; describes what this space is for  |
| `skey`           | string        | Yes      | Space key; differentiates spaces of the same type |
| `displayName`    | string        | No       | Human-readable name                               |
| `description`    | string        | No       | Description of the space                          |
| `accessMode`     | string        | No       | `default_allow` (default) or `default_deny`       |
| `managingAppDid` | string        | No       | DID of the application that manages this space    |
| `config`         | object        | No       | Space configuration (see below)                   |

**Response (201):**

```json
{
  "uri": "ats://did:plc:abc123/com.example.forum/main"
}
```

The creator is automatically added as a write member. Use [`dev.happyview.space.getSpace`](#getting-a-space) to retrieve the full space object.

### Space configuration

The `config` object supports:

| Field              | Type    | Default | Description                                               |
| ------------------ | ------- | ------- | --------------------------------------------------------- |
| `membershipPublic` | boolean | `false` | Whether the member list is visible without authentication |
| `recordsPublic`    | boolean | `false` | Whether records are readable without membership           |

Additional fields are preserved as-is.

## Getting a space

```ts tab="TypeScript" tab-group="language"
const response = await fetch(
  "https://happyview.example.com/xrpc/dev.happyview.space.getSpace?space=ats://did:plc:abc123/com.example.forum/main",
  {
    headers: {
      "X-Client-Key": CLIENT_KEY,
      "Authorization": `DPoP ${ACCESS_TOKEN}`,
      "DPoP": DPOP_PROOF,
    },
  },
);
interface Space {
  uri: string;
  isOwner: boolean;
}
const data: Space = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch(
  "https://happyview.example.com/xrpc/dev.happyview.space.getSpace?space=ats://did:plc:abc123/com.example.forum/main",
  {
    headers: {
      "X-Client-Key": CLIENT_KEY,
      "Authorization": `DPoP ${ACCESS_TOKEN}`,
      "DPoP": DPOP_PROOF,
    },
  },
);
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .get("https://happyview.example.com/xrpc/dev.happyview.space.getSpace")
    .query(&[("space", "ats://did:plc:abc123/com.example.forum/main")])
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("DPoP {}", access_token))
    .header("DPoP", &dpop_proof)
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
req, _ := http.NewRequest("GET",
  "https://happyview.example.com/xrpc/dev.happyview.space.getSpace?space=ats://did:plc:abc123/com.example.forum/main",
  nil)
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "DPoP "+accessToken)
req.Header.Set("DPoP", dpopProof)
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl 'https://happyview.example.com/xrpc/dev.happyview.space.getSpace?space=ats://did:plc:abc123/com.example.forum/main' \
  -H 'X-Client-Key: hvc_...' \
  -H 'Authorization: DPoP <token>' \
  -H 'DPoP: <proof>'
```

If `membershipPublic` is `false`, the caller must be authenticated and be a member (or the owner) to see the space. Non-members receive a `404 Not Found`.

## Listing spaces

Returns spaces where the authenticated user is a member.

```ts tab="TypeScript" tab-group="language"
const response = await fetch(
  "https://happyview.example.com/xrpc/dev.happyview.space.listSpaces?limit=20",
  {
    headers: {
      "X-Client-Key": CLIENT_KEY,
      "Authorization": `DPoP ${ACCESS_TOKEN}`,
      "DPoP": DPOP_PROOF,
    },
  },
);
interface Space {
  uri: string;
  isOwner: boolean;
}
interface ListSpacesResponse {
  spaces: Space[];
  cursor?: string;
}
const data: ListSpacesResponse = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch(
  "https://happyview.example.com/xrpc/dev.happyview.space.listSpaces?limit=20",
  {
    headers: {
      "X-Client-Key": CLIENT_KEY,
      "Authorization": `DPoP ${ACCESS_TOKEN}`,
      "DPoP": DPOP_PROOF,
    },
  },
);
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .get("https://happyview.example.com/xrpc/dev.happyview.space.listSpaces")
    .query(&[("limit", "20")])
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("DPoP {}", access_token))
    .header("DPoP", &dpop_proof)
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
req, _ := http.NewRequest("GET",
  "https://happyview.example.com/xrpc/dev.happyview.space.listSpaces?limit=20",
  nil)
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "DPoP "+accessToken)
req.Header.Set("DPoP", dpopProof)
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl 'https://happyview.example.com/xrpc/dev.happyview.space.listSpaces?limit=20' \
  -H 'X-Client-Key: hvc_...' \
  -H 'Authorization: DPoP <token>' \
  -H 'DPoP: <proof>'
```

**Parameters:**

| Field    | Type    | Required | Default | Description                  |
| -------- | ------- | -------- | ------- | ---------------------------- |
| `limit`  | integer | No       | 50      | Max spaces to return (1-100) |
| `cursor` | string  | No       |         | Pagination cursor            |

**Response:**

```json
{
  "spaces": [
    {
      "uri": "ats://did:plc:abc123/com.example.forum/main",
      "isOwner": true
    }
  ],
  "cursor": "MjAyNi0wNS0wOVQxMjowMDowMFp8YXRzOi8vZGlkOnBsYzphYmMxMjMvY29tLmV4YW1wbGUuZm9ydW0vbWFpbg"
}
```

## Updating a space

Only the space owner or a HappView super admin can update a space.

```ts tab="TypeScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.updateSpace", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    "Authorization": `DPoP ${ACCESS_TOKEN}`,
    "DPoP": DPOP_PROOF,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    space: "ats://did:plc:abc123/com.example.forum/main",
    displayName: "Updated Forum Name",
    accessMode: "default_deny",
    appAllowlist: ["did:web:myapp.example.com"],
  }),
});
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.updateSpace", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    "Authorization": `DPoP ${ACCESS_TOKEN}`,
    "DPoP": DPOP_PROOF,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    space: "ats://did:plc:abc123/com.example.forum/main",
    displayName: "Updated Forum Name",
    accessMode: "default_deny",
    appAllowlist: ["did:web:myapp.example.com"],
  }),
});
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("https://happyview.example.com/xrpc/dev.happyview.space.updateSpace")
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("DPoP {}", access_token))
    .header("DPoP", &dpop_proof)
    .json(&serde_json::json!({
        "space": "ats://did:plc:abc123/com.example.forum/main",
        "displayName": "Updated Forum Name",
        "accessMode": "default_deny",
        "appAllowlist": ["did:web:myapp.example.com"]
    }))
    .send()
    .await?;
```
```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{
  "space": "ats://did:plc:abc123/com.example.forum/main",
  "displayName": "Updated Forum Name",
  "accessMode": "default_deny",
  "appAllowlist": ["did:web:myapp.example.com"]
}`)
req, _ := http.NewRequest("POST",
  "https://happyview.example.com/xrpc/dev.happyview.space.updateSpace", body)
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "DPoP "+accessToken)
req.Header.Set("DPoP", dpopProof)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST 'https://happyview.example.com/xrpc/dev.happyview.space.updateSpace' \
  -H 'X-Client-Key: hvc_...' \
  -H 'Authorization: DPoP <token>' \
  -H 'DPoP: <proof>' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": "ats://did:plc:abc123/com.example.forum/main",
    "displayName": "Updated Forum Name",
    "accessMode": "default_deny",
    "appAllowlist": ["did:web:myapp.example.com"]
  }'
```

All fields except `space` are optional. Only provided fields are updated. To clear an optional field, pass `null`.

## Deleting a space

Only the space owner or a HappyView super admin can delete a space.

```ts tab="TypeScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.deleteSpace", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    "Authorization": `DPoP ${ACCESS_TOKEN}`,
    "DPoP": DPOP_PROOF,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    space: "ats://did:plc:abc123/com.example.forum/main",
  }),
});
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.deleteSpace", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    "Authorization": `DPoP ${ACCESS_TOKEN}`,
    "DPoP": DPOP_PROOF,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    space: "ats://did:plc:abc123/com.example.forum/main",
  }),
});
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("https://happyview.example.com/xrpc/dev.happyview.space.deleteSpace")
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("DPoP {}", access_token))
    .header("DPoP", &dpop_proof)
    .json(&serde_json::json!({
        "space": "ats://did:plc:abc123/com.example.forum/main"
    }))
    .send()
    .await?;
```
```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{"space": "ats://did:plc:abc123/com.example.forum/main"}`)
req, _ := http.NewRequest("POST",
  "https://happyview.example.com/xrpc/dev.happyview.space.deleteSpace", body)
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "DPoP "+accessToken)
req.Header.Set("DPoP", dpopProof)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST 'https://happyview.example.com/xrpc/dev.happyview.space.deleteSpace' \
  -H 'X-Client-Key: hvc_...' \
  -H 'Authorization: DPoP <token>' \
  -H 'DPoP: <proof>' \
  -H 'Content-Type: application/json' \
  -d '{"space": "ats://did:plc:abc123/com.example.forum/main"}'
```

<Callout type="warn">
Deleting a space does not currently cascade to records, members, or credentials. This behavior may change.
</Callout>
