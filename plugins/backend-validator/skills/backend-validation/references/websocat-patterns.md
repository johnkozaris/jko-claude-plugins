# websocat probe patterns

Discover the application's message protocol before probing. A minimal
send-one/receive-one shape is:

```bash
printf '%s\n' "$WS_MESSAGE" | websocat -n1 \
  -H "Authorization: ******" \
  ${WS_SUBPROTOCOL:+--protocol "$WS_SUBPROTOCOL"} \
  "$WS_URL"
```

Confirm current flag syntax with `websocat --help`. `-1` means one message and
`-n` suppresses a close frame on stdin EOF in current websocat releases; those
semantics may not fit every server.

Set Origin only when project configuration requires it. Do not invent a
localhost Origin. Use a bounded command timeout so a silent server cannot hold
the validation indefinitely.

Interpret evidence narrowly:

- handshake failure distinguishes transport/auth/subprotocol problems;
- a local send succeeds before application acceptance is known;
- no reply may be correct for a one-way protocol;
- verbose output can expose Authorization and must be redacted.

For mTLS, reconnect, binary frames, and keepalive, inspect the installed
websocat help and project protocol instead of copying version-sensitive flags.
