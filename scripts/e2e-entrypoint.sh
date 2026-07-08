#!/bin/sh
set -e

# Wait for Caddy's internal CA certificate to appear in the shared volume,
# then install it so reqwest (native-tls / OpenSSL) trusts TLS connections
# proxied through Caddy (e.g. the PDS OAuth endpoints).
#
# Reading Caddy's CA (its pki/ tree is root-owned, mode 0700 to protect the CA
# key) and installing it into the system trust store both require root. The
# image now runs as the non-root `app` user, so the e2e compose overrides the
# container user back to root for this bootstrap; we install the CA as root and
# then drop to `app` to run the server, keeping the non-root runtime under test.
CA_CERT=/caddy-data/caddy/pki/authorities/local/root.crt
if [ -d /caddy-data ]; then
  echo "Waiting for Caddy CA certificate..."
  while [ ! -f "$CA_CERT" ]; do sleep 0.5; done
  cp "$CA_CERT" /usr/local/share/ca-certificates/caddy-local.crt
  update-ca-certificates 2>/dev/null
  echo "Caddy CA certificate installed."
fi

# Drop back to the unprivileged app user (uid 10001) to run the server when we
# started as root; run directly otherwise (already unprivileged).
if [ "$(id -u)" = "0" ]; then
  exec runuser -u app -- /entrypoint.sh "$@"
fi
exec /entrypoint.sh "$@"
