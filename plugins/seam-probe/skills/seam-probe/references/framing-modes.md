# Framing modes

Length-prefixed framing means: every frame on the wire starts with a
length header that says how many bytes follow. The probe wraps the
underlying `tokio::net::UnixStream` in a `LengthDelimitedCodec`
according to your `--framing` flag.

## `be32` (default)

4-byte big-endian unsigned length, then payload.

```
[ 00 00 00 0c | "hello world!" ]
  └ length=12 ┘└ 12-byte body  ┘
```

This is the de-facto standard. Used by gRPC, many Rust RPC servers,
many Go RPC servers. Try this first.

## `be64`

8-byte big-endian unsigned length, then payload. Same idea as be32 but
allows individual frames > 4 GiB. Rare in practice; you'd see it in
servers that move large blobs (file transfers, ML model uploads).

## `varint`

**Not implemented.** Reserved name. If you need varint framing,
use `--framing none` and prepend the LEB128 length to your payload
manually:

```bash
# 12-byte body → varint prefix is 0x0c
echo '{"op":"raw","hex":"0c68656c6c6f20776f726c6421"}' | \
  seam-probe socket --path /tmp/foo.sock --framing none
```

## `none`

Raw bytes. The probe writes whatever you give it, in order, without
prefixing anything. Reads also pass through unbuffered — the inbound
`frame` events will arrive in chunks of up to 8 KiB depending on TCP
delivery (UDS is similar).

Useful when:
- The protocol has no length prefix (line-delimited, framed by `\n`).
- The protocol uses non-supported framing (varint, COBS, custom).
- You want to fuzz byte sequences without the codec rejecting them.

## Maximum payload size

Framed reads, stdin command lines, and outbound payloads are capped at
**8 MiB**. The probe rejects a larger length header, command, or payload.
Raw inbound reads arrive in bounded 8 KiB chunks. Raise `MAX_FRAME_BYTES`
in `crate/src/socket.rs` if a trusted protocol genuinely needs more.

## Why no little-endian variants?

Almost no real-world UDS protocols use little-endian length prefixes —
network byte order convention dominates. If you encounter one in the
wild, file a feature request.
