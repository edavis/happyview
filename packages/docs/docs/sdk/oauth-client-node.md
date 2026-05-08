# Node Client

The Node client handles OAuth authorization and callback flows for server-side Node.js apps authenticating with a HappyView instance. It wraps the [OAuth Client](./oauth-client.md) with handle/DID resolution and a server-friendly API that returns authorization URLs instead of redirecting the browser.

## Installation

```bash
npm install @happyview/oauth-client-node
```

## Setup

```typescript
import { HappyViewNodeClient } from "@happyview/oauth-client-node";

const client = new HappyViewNodeClient({
  instanceUrl: "https://happyview.example.com",
  clientId: "https://example.com/oauth-client-metadata.json",
  clientKey: "hvc_your_client_key",
  clientSecret: "hvs_your_secret", // optional, for confidential clients
  redirectUri: "https://example.com/oauth/callback",
  storage: myStorageAdapter,
});
```

| Option        | Required | Description                                                                  |
| ------------- | -------- | ---------------------------------------------------------------------------- |
| `instanceUrl` | Yes      | The HappyView instance URL                                                   |
| `clientId`    | Yes      | URL where your app serves its [OAuth client metadata](#oauth-client-metadata) |
| `clientKey`   | Yes      | API client key from the HappyView admin dashboard                            |
| `clientSecret`| No       | API client secret — makes this a confidential client                         |
| `redirectUri` | Yes      | OAuth callback URL for your server                                           |
| `scopes`      | No       | OAuth scopes to request. Defaults to `"atproto"`                             |
| `storage`     | Yes      | Storage adapter for persisting sessions and pending auth state               |
| `sessionHooks`| No       | Hooks called on session lifecycle events                                     |
| `fetch`       | No       | Custom fetch implementation                                                  |

:::note
Unlike the browser client, `storage` is required — there is no default. Use any adapter that implements `StorageAdapter` (e.g., backed by Redis, a database, or the filesystem).
:::

## Authorization

`authorize()` resolves the user's handle, discovers their PDS, provisions a DPoP key, and returns an authorization URL. Your server should redirect the user to this URL:

```typescript
// In your login route handler
const url = await client.authorize("alice.bsky.social");
res.redirect(url.toString());
```

### Authorization options

Pass options to customize the authorization request:

```typescript
const url = await client.authorize("alice.bsky.social", {
  scope: "atproto transition:generic",
  state: myCustomState,
  redirect_uri: "https://example.com/alt-callback",
  display: "popup",
  prompt: "consent",
  ui_locales: "en",
});
```

| Option        | Description                                                  |
| ------------- | ------------------------------------------------------------ |
| `scope`       | Override the default scopes for this request                 |
| `state`       | Custom state value (defaults to a random hex string)         |
| `redirect_uri`| Override the default redirect URI for this request           |
| `display`     | `"page"`, `"popup"`, `"touch"`, or `"wap"`                  |
| `prompt`      | Prompt behavior (e.g., `"consent"`, `"login"`)               |
| `nonce`       | Nonce for ID token validation                                |
| `max_age`     | Maximum authentication age in seconds                        |
| `ui_locales`  | Preferred UI languages                                       |
| `signal`      | AbortSignal to cancel the request                            |

## Callback

In your OAuth callback route, pass the query parameters to `callback()`:

```typescript
// In your callback route handler
const params = new URLSearchParams(req.url.split("?")[1]);
const { session, state } = await client.callback(params);

// session is ready to use
console.log(session.did);
console.log(session.scopes);
```

The returned `HappyViewSession` is persisted to storage and ready for authenticated requests.

You can override the redirect URI if needed:

```typescript
const { session } = await client.callback(params, {
  redirect_uri: "https://example.com/alt-callback",
});
```

## Checking approved scopes

After authorization or session restoration, you can check which scopes were approved:

```typescript
console.log(session.scopes);
// ["atproto", "transition:generic"]
```

To fetch the latest scopes from the server:

```typescript
const info = await client.getSession("did:plc:abc123");
console.log(info.scopes);
// ["atproto", "transition:generic"]
```

## Authenticated requests

The session's `fetchHandler` attaches DPoP proof headers automatically:

```typescript
const response = await session.fetchHandler(
  "/xrpc/com.example.getStuff?limit=10",
  { method: "GET" },
);

const data = await response.json();
```

Pass a relative path (prepends the HappyView instance URL) or a full URL (used as-is).

## Session restoration

Restore a previously stored session by DID:

```typescript
const session = await client.restore("did:plc:abc123");
```

Unlike the browser client, `restore()` requires a DID — there is no "last active" session concept on the server.

Throws `InvalidStateError` if no session is found for the given DID.

## Revoke session

```typescript
await client.revoke("did:plc:abc123");
```

## Aborting a pending authorization

If the user abandons the login flow, clean up the pending state:

```typescript
await client.abortRequest(authorizationUrl);
```

## OAuth client metadata

Your app must serve an OAuth client metadata JSON document at the URL you pass as `clientId`. The PDS fetches this during authorization to validate the redirect URI and display your app's information.

```typescript
// Express example
app.get("/oauth-client-metadata.json", (req, res) => {
  const origin = `${req.protocol}://${req.get("host")}`;
  res.json({
    client_id: `${origin}/oauth-client-metadata.json`,
    client_name: "My App",
    client_uri: origin,
    redirect_uris: [`${origin}/oauth/callback`],
    token_endpoint_auth_method: "none",
    grant_types: ["authorization_code", "refresh_token"],
    scope: "atproto",
    application_type: "web",
    dpop_bound_access_tokens: true,
  });
});
```

The `redirect_uris` array must include the `redirectUri` your client is configured with.

## Re-exports

This package re-exports everything from `@happyview/oauth-client`, so you don't need to install the core package separately:

```typescript
import {
  HappyViewNodeClient,
  HappyViewSession,
  ApiError,
  type StorageAdapter,
} from "@happyview/oauth-client-node";
```

It also re-exports handle and DID resolution utilities from `@atproto-labs/handle-resolver` and `@atproto-labs/did-resolver`.

## Error handling

All errors extend `HappyViewError`. The Node client additionally uses `OAuthCallbackError` for callback failures:

| Error | When |
| --- | --- |
| `ApiError` | HappyView API returned a non-OK response (has `status` and `body`) |
| `OAuthCallbackError` | OAuth callback failed — wraps the callback params and underlying error |
| `InvalidStateError` | Missing or invalid OAuth/session state |
| `TokenExchangeError` | Token exchange with the PDS failed (has `status` and `body`) |
| `ResolutionError` | Handle or DID resolution failed |

```typescript
import {
  OAuthCallbackError,
  InvalidStateError,
} from "@happyview/oauth-client-node";

try {
  const { session } = await client.callback(params);
} catch (err) {
  if (err instanceof OAuthCallbackError) {
    console.error("OAuth callback failed:", err.message);
    console.error("State:", err.state);
  }
}
```
