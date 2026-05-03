# OIDC Token Flows

Choose the right grant for the situation. Most mistakes come from trying the wrong one.

## Authorization Code + PKCE (recommended for public clients)

**Use when:** your app's runtime client is a browser SPA, mobile app, or any "public" client that can't safely store a secret. This is what the validation script uses.

```bash
oauth2c https://auth.example.com/application/o/your-app/ \
  --client-id your-public-client-id \
  --auth-method none \
  --grant-type authorization_code \
  --response-types code \
  --response-mode query \
  --pkce \
  --prompt login \
  --scopes "openid profile email offline_access"
```

- `--auth-method none` = public client (no client_secret)
- `--pkce` = PKCE challenge required for security
- `--prompt login` = force fresh authentication even if a session exists (avoids picking up admin sessions)
- `offline_access` scope is required to receive a `refresh_token`

First run opens a browser; you log in; oauth2c captures the callback at `http://localhost:9876/callback` (Authentik and others require this exact URI to be in the client's allowed redirects).

## Refresh Token (silent renewal)

```bash
curl -sS -X POST https://auth.example.com/application/o/your-app/token/ \
  --data-urlencode "grant_type=refresh_token" \
  --data-urlencode "refresh_token=$REFRESH_TOKEN" \
  --data-urlencode "client_id=your-public-client-id" \
  --data-urlencode "scope=openid profile email offline_access" \
  | jq -r '.access_token'
```

- No client_secret for public clients
- Many providers (Authentik, Auth0) rotate the refresh_token on each use — capture the new one
- Refresh tokens typically last 30 days; re-auth via browser when they expire

## Client Credentials (machine-to-machine)

**Use when:** the caller is a server or automated job, not a user. Requires a **confidential** client.

```bash
oauth2c https://auth.example.com/application/o/your-service/ \
  --client-id your-confidential-client \
  --client-secret "$CLIENT_SECRET" \
  --grant-type client_credentials \
  --scopes "api.read api.write"
```

Token has no user context — only client identity. Endpoints requiring `sub` claim (user ID) won't work with this.

## ROPC (Resource Owner Password Credentials)

**Do not use.** Deprecated in OAuth 2.1. Most providers refuse it on public clients. Even on confidential clients, it exposes user passwords to your script. Authorization-code + PKCE is strictly better for every use case where ROPC would work.

Symptom: providers return `invalid_grant` with identical error text for both wrong credentials and missing client support — impossible to debug from the client side.

## Device Code

**Use when:** no browser on the device (CLI tool on a headless server, IoT device, smart TV).

```bash
oauth2c https://auth.example.com/application/o/your-app/ \
  --client-id your-public-client \
  --grant-type urn:ietf:params:oauth:grant-type:device_code \
  --scopes "openid profile"
```

Prints a code + URL. User visits URL on another device, enters code, script polls token endpoint until authorized.

## Debugging tokens

```bash
# Decode JWT claims
echo "$TOKEN" | awk -F. '{print $2}' | base64 -d 2>/dev/null | jq

# Verify specific claims
echo "$TOKEN" | awk -F. '{print $2}' | base64 -d 2>/dev/null \
  | jq '{iss, aud, sub, preferred_username, email_verified, exp}'

# Check expiry
echo "$TOKEN" | awk -F. '{print $2}' | base64 -d 2>/dev/null \
  | jq -r '"exp: " + (.exp | todate)'
```

Common 401 causes:
1. `exp` in the past — token expired, refresh it
2. `iss` doesn't match backend's configured authority — wrong tenant or wrong app
3. `aud` missing or wrong — backend's `Audiences` list doesn't include this client
4. `email_verified: false` — backend rejects unverified emails (add attribute in user profile)
5. Clock skew — client's time is off; NTP-sync and retry

## Discovery endpoint

Every OIDC provider publishes config at `/.well-known/openid-configuration`:

```bash
curl -s https://auth.example.com/application/o/your-app/.well-known/openid-configuration \
  | jq '{issuer, token_endpoint, authorization_endpoint, grant_types_supported, scopes_supported}'
```

Critical when onboarding a new provider — confirms the issuer URL, advertises supported grants, and lists scopes. `grant_types_supported` can lie — it shows what the server software supports, not what this specific client is allowed to use.
