---
title: "Credentials"
---

<Callout type="error" title="Experimental">
This API is experimental and will change. See the [Permissioned Spaces overview](../spaces.md) for context.
</Callout>

Space credentials are short-lived JWTs for cross-service access to space data. A member proves their membership to get a grant, exchanges the grant for a credential JWT, then passes it to an external service that needs to read the space's records.

## How credentials work

Credential issuance is a two-step process:

```mermaid
sequenceDiagram
    participant App as Client App
    participant HV as HappyView
    participant Svc as External Service

    App->>HV: POST dev.happyview.space.getMemberGrant<br/>(DPoP auth, must be a member)
    HV->>HV: Verify membership
    HV-->>App: grant token + expiresAt

    App->>HV: POST dev.happyview.space.getSpaceCredential<br/>(DPoP auth, grant token)
    HV->>HV: Verify grant<br/>Check app access (allow/deny list)<br/>Sign credential with space keypair
    HV-->>App: credential JWT + expiresAt

    App->>Svc: Request with Authorization: Bearer credential
    Svc->>HV: Read space records<br/>(Bearer credential)
    HV->>HV: Verify credential signature<br/>via space DID doc
    HV-->>Svc: Record data
```

Credentials are ES256 JWTs signed with a P-256 keypair unique to each space. The keypair is generated on first credential request and stored encrypted (AES-256-GCM).

## Step 1: Get a member grant

The caller must be an authenticated member of the space. The grant is a short-lived token (5 minutes) that proves membership.

```ts tab="TypeScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.getMemberGrant", {
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
interface GrantResponse {
  grant: string;
  expiresAt: string;
}
const data: GrantResponse = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.getMemberGrant", {
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
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("https://happyview.example.com/xrpc/dev.happyview.space.getMemberGrant")
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("DPoP {}", access_token))
    .header("DPoP", &dpop_proof)
    .json(&serde_json::json!({
        "space": "ats://did:plc:abc123/com.example.forum/main"
    }))
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{"space": "ats://did:plc:abc123/com.example.forum/main"}`)
req, _ := http.NewRequest("POST",
  "https://happyview.example.com/xrpc/dev.happyview.space.getMemberGrant", body)
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "DPoP "+accessToken)
req.Header.Set("DPoP", dpopProof)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST 'https://happyview.example.com/xrpc/dev.happyview.space.getMemberGrant' \
  -H 'X-Client-Key: hvc_...' \
  -H 'Authorization: DPoP <token>' \
  -H 'DPoP: <proof>' \
  -H 'Content-Type: application/json' \
  -d '{
    "space": "ats://did:plc:abc123/com.example.forum/main"
  }'
```

**Response:**

```json
{
  "grant": "eyJhbGciOiJIUzI1NiJ9...",
  "expiresAt": "2026-05-09T12:05:00Z"
}
```

## Step 2: Get a space credential

Exchange the grant for a space credential JWT. The credential is signed by the space's keypair and has a 4-hour TTL.

```ts tab="TypeScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.getSpaceCredential", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    "Authorization": `DPoP ${ACCESS_TOKEN}`,
    "DPoP": DPOP_PROOF,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    grant: "eyJhbGciOiJIUzI1NiJ9...",
  }),
});
interface CredentialResponse {
  credential: string;
  expiresAt: string;
}
const data: CredentialResponse = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch("https://happyview.example.com/xrpc/dev.happyview.space.getSpaceCredential", {
  method: "POST",
  headers: {
    "X-Client-Key": CLIENT_KEY,
    "Authorization": `DPoP ${ACCESS_TOKEN}`,
    "DPoP": DPOP_PROOF,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    grant: "eyJhbGciOiJIUzI1NiJ9...",
  }),
});
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .post("https://happyview.example.com/xrpc/dev.happyview.space.getSpaceCredential")
    .header("X-Client-Key", client_key)
    .header("Authorization", format!("DPoP {}", access_token))
    .header("DPoP", &dpop_proof)
    .json(&serde_json::json!({
        "grant": "eyJhbGciOiJIUzI1NiJ9..."
    }))
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
body := bytes.NewBufferString(`{"grant": "eyJhbGciOiJIUzI1NiJ9..."}`)
req, _ := http.NewRequest("POST",
  "https://happyview.example.com/xrpc/dev.happyview.space.getSpaceCredential", body)
req.Header.Set("X-Client-Key", clientKey)
req.Header.Set("Authorization", "DPoP "+accessToken)
req.Header.Set("DPoP", dpopProof)
req.Header.Set("Content-Type", "application/json")
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl -X POST 'https://happyview.example.com/xrpc/dev.happyview.space.getSpaceCredential' \
  -H 'X-Client-Key: hvc_...' \
  -H 'Authorization: DPoP <token>' \
  -H 'DPoP: <proof>' \
  -H 'Content-Type: application/json' \
  -d '{
    "grant": "eyJhbGciOiJIUzI1NiJ9..."
  }'
```

**Response:**

```json
{
  "credential": "eyJhbGciOiJFUzI1NiJ9...",
  "expiresAt": "2026-05-09T16:00:00Z"
}
```

### Credential claims

The JWT payload contains:

| Claim | Description |
|---|---|
| `iss` | The space's DID (who signed it) |
| `sub` | The member's DID (who it was issued to) |
| `space` | The full `ats://` space URI |
| `scope` | Access level (`read`) |
| `iat` | Issued at (Unix timestamp) |
| `exp` | Expiry (Unix timestamp) |

## Using a credential

Pass the credential as a standard Bearer token in the `Authorization` header. HappyView distinguishes space credentials from other tokens by checking the JWT header's `typ` field (`space_credential`).

```ts tab="TypeScript" tab-group="language"
const response = await fetch(
  "https://happyview.example.com/xrpc/dev.happyview.space.getRecord?space=...&collection=...&rkey=...",
  {
    headers: {
      "Authorization": `Bearer ${SPACE_CREDENTIAL}`,
    },
  },
);
const data = await response.json();
```
```js tab="JavaScript" tab-group="language"
const response = await fetch(
  "https://happyview.example.com/xrpc/dev.happyview.space.getRecord?space=...&collection=...&rkey=...",
  {
    headers: {
      "Authorization": `Bearer ${SPACE_CREDENTIAL}`,
    },
  },
);
const data = await response.json();
```
```rust tab="Rust" tab-group="language"
let response = client
    .get("https://happyview.example.com/xrpc/dev.happyview.space.getRecord")
    .query(&[("space", "..."), ("collection", "..."), ("rkey", "...")])
    .header("Authorization", format!("Bearer {}", space_credential))
    .send()
    .await?;
let data: serde_json::Value = response.json().await?;
```
```go tab="Go" tab-group="language"
req, _ := http.NewRequest("GET",
  "https://happyview.example.com/xrpc/dev.happyview.space.getRecord?space=...&collection=...&rkey=...",
  nil)
req.Header.Set("Authorization", "Bearer "+spaceCredential)
resp, err := http.DefaultClient.Do(req)
```
```sh tab="cURL" tab-group="language"
curl 'https://happyview.example.com/xrpc/dev.happyview.space.getRecord?space=...&collection=...&rkey=...' \
  -H 'Authorization: Bearer eyJhbGciOiJFUzI1NiIsInR5cCI6InNwYWNlX2NyZWRlbnRpYWwifQ...'
```

No DPoP auth or client key is needed when authenticating via space credential — the credential itself is sufficient. The user's identity comes from the `sub` claim in the JWT.

HappyView verifies the credential by resolving the issuer's DID document, extracting the signing key, and validating the JWT signature and expiry. If valid, the request is treated as if the credential's `sub` is a member of the space.

## App access control

Before issuing a credential, HappyView checks whether the calling app (identified by its DPoP client key) is allowed to access the space:

- **`default_allow` mode**: any app can get credentials unless it's on the `appDenylist`
- **`default_deny` mode**: only apps on the `appAllowlist` can get credentials

If no client key is present in the DPoP claims, the check is skipped (direct user access without an app intermediary).

## External credential verification

HappyView can also verify credentials issued by *other* HappyView instances or space-aware services. When a Bearer space credential is presented, HappyView:

1. Decodes the JWT without verification to extract the `iss` (issuer DID)
2. Resolves the issuer's DID document
3. Extracts the signing key from the DID doc
4. Verifies the JWT signature and expiry
5. Checks that the `space` claim matches the requested space

A credential issued by one instance can be used to read from another instance that hosts the same space's data.
