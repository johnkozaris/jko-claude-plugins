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

Use it only when the target's read loop or protocol contract shows a 32-bit
big-endian length.

## `be64`

8-byte big-endian unsigned length, then payload. The wire field can represent
larger lengths, but this probe still caps every frame at 8 MiB.

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

Little-endian length fields are not implemented. Inspect the target read loop
instead of relying on protocol-frequency guesses.
