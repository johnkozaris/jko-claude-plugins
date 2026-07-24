# Discovering an unfamiliar UDS protocol

The probe gives you four framing modes:

- `be32` — 4-byte big-endian length prefix (gRPC, most line-of-business
  RPC servers).
- `be64` — 8-byte big-endian length prefix (very-large-frame variants).
- `varint` — protobuf-style LEB128 length prefix. **Not implemented in
  the current release**; use `none` and build the prefix yourself.
- `none` — raw bytes, no framing. socat-equivalent.

## 1. Find where the server binds

Grep the codebase for `UnixListener::bind`, `unix_socket`, `bind(.*\.sock)`,
or platform equivalents. The path you find is what you pass to
`--path`.

## 2. Find where the server reads

Identify the server's read loop. Look for:

- Tokio: `LengthDelimitedCodec::builder()` and the `length_field_type`
  call. `length_field_type::<u32>().big_endian()` → `be32`. `<u64>` →
  `be64`. Any `.little_endian()` → not supported by seam-probe yet.
- Manual `read_exact`: count bytes. 4 → `be32`. 8 → `be64`. Variable →
  probably varint.
- No length read, just `read_to_end` or `read_buf` → `none`.

## 3. Try the most likely framing first

```bash
echo '{"op":"send","payload":{"hello":"world"}}' | \
  seam-probe socket --path /tmp/foo.sock --framing be32
```

If the server processes the frame, you'll see it in its logs (or the
server emits a reply that arrives as a `frame` event).

If the server hangs forever after your `send`, you've over-counted: try
`be64`. If the server closes the connection, you've under-counted or
the framing is wrong: try `none` and inspect raw bytes.

## 4. Discover payload shape

Once framing is right, dump a recorded capture into the probe and look
at the inbound `frame` events. The probe attempts JSON decode; if
`payload` is present in the event line, the wire is JSON. If only
`hex` is present, it's binary (protobuf, msgpack, custom).

## 5. Common gotchas

- **Path length on macOS** — Unix-domain socket paths are bounded at
  104 bytes. Ensure your path is short enough.
- **Permissions** — `EACCES` means the server's socket is owned by a
  different user or has restrictive mode bits. Check with `ls -la`.
- **Header byte ordering** — `be32` is what 99% of Rust/Tokio servers
  use. If the Rust source explicitly says `little_endian()`, you need a
  custom framing path: file a feature request.
- **SOCK_SEQPACKET vs SOCK_STREAM** — the probe currently uses
  `SOCK_STREAM`. SEQPACKET endpoints will reject the connection with
  `EPROTOTYPE`. File a feature request if you need it.
