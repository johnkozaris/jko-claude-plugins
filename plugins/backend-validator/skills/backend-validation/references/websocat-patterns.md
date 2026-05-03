# websocat Patterns

All examples assume `$TOKEN` is set from the token-acquisition script.

## Send-one-receive-one probe

```bash
echo '{"op":"ping"}' | websocat -n1 \
  -H "Authorization: Bearer $TOKEN" \
  ws://localhost:5000/endpoint
```

`-n1` = exit after one message. Good for scripted smoke tests — exit 0 means handshake succeeded AND a message came back.

## Streaming messages

```bash
websocat -H "Authorization: Bearer $TOKEN" \
  ws://localhost:5000/stream
```

Stays open until stdin closes or server terminates. Each incoming message prints as a line. Pipe into `jq -c` for structured parsing:

```bash
websocat -H "Authorization: Bearer $TOKEN" ws://localhost:5000/stream \
  | while read line; do
      echo "$line" | jq -r '.type + " " + (.ts | tostring)'
    done
```

## Subprotocols

```bash
# GraphQL over WebSocket
websocat --protocol graphql-transport-ws \
  -H "Authorization: Bearer $TOKEN" \
  wss://api.example.com/graphql

# MQTT over WebSocket
websocat --protocol mqtt \
  wss://broker.example.com/mqtt
```

## Ping/pong keepalive

```bash
websocat --ping-interval 30 --ping-timeout 10 \
  -H "Authorization: Bearer $TOKEN" \
  wss://api.example.com/stream
```

Sends a WS ping every 30s; disconnects if no pong within 10s. Useful for long-running streams over flaky networks.

## Auto-reconnect

```bash
websocat "autoreconnect:wss://api.example.com/stream" \
  -H "Authorization: Bearer $TOKEN"
```

Socat-style URL specifier. Reconnects with exponential backoff on drop.

## Text vs binary

Default text mode. For binary frames:

```bash
websocat --binary -H "Authorization: Bearer $TOKEN" ws://...
```

## mTLS (client cert)

websocat's `--tls-client-cert` flag exists but is rough. For serious mTLS, use `wscat`:

```bash
wscat -c wss://api.example.com/stream \
  --ca ca.pem --cert client.crt --key client.key \
  -H "Authorization: Bearer $TOKEN"
```

`wscat` is `pnpm dlx wscat` — slower startup than websocat, but cleaner mTLS.

## Debug handshake

```bash
websocat -v -H "Authorization: Bearer $TOKEN" wss://api.example.com/
```

`-v` prints handshake request/response headers. Essential when debugging origin-check rejections, subprotocol negotiation failures, or TLS issues.

## Close codes

```bash
# Send a close frame with specific code
websocat -n1 --close-code 1000 --close-reason "bye" ws://...
```

Server-side validation of graceful shutdown.

## Common gotchas

- **Origin rejection** — servers often require `Origin:` header. Pass with `-H "Origin: http://localhost:3000"`.
- **EOF behavior** — `-E` / `--exit-on-eof` exits when stdin closes. Without it, websocat stays open indefinitely.
- **curl has `--ws` too** — experimental. Fine for handshake sanity; websocat is better for piping.
