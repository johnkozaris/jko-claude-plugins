# Serde

## Core Rule: Always Derive

`#[derive(Serialize, Deserialize)]` handles the vast majority of cases. Implement manually only when the serialized shape must differ significantly from the Rust layout, or for orphan rule workarounds.

## Essential Attributes

| Attribute | Use |
|---|---|
| `#[serde(rename = "type")]` | Match external API field names, escape Rust keywords |
| `#[serde(rename_all = "camelCase")]` | Bulk rename for naming convention mismatch |
| `#[serde(default)]` | Accept missing fields without `Option<T>` |
| `#[serde(skip_serializing_if = "Option::is_none")]` | Omit nulls from output |
| `#[serde(skip)]` | Exclude field from both ser/de |
| `#[serde(flatten)]` | Inline nested struct fields into parent |
| `#[serde(with = "module")]` | Custom ser/de module |
| `#[serde(deny_unknown_fields)]` | Strict: reject unexpected fields |

## Enum Representations

| Representation | Attribute | Output |
|---|---|---|
| Externally tagged (default) | — | `{"Variant": {...}}` |
| Internally tagged | `#[serde(tag = "type")]` | `{"type": "Variant", ...}` |
| Adjacently tagged | `#[serde(tag = "t", content = "c")]` | `{"t": "V", "c": {...}}` |
| Untagged | `#[serde(untagged)]` | `{...}` (tries variants in order) |

Prefer internally or adjacently tagged for human-readable APIs. Untagged as last resort — overhead and cryptic errors.

## Zero-Copy Deserialization

Use `&str` instead of `String` to borrow directly from the input buffer:

```rust
#[derive(Deserialize)]
struct Record<'a> {
    #[serde(borrow)]
    name: &'a str,        // zero-copy: borrows from input
    value: Cow<'a, str>,  // borrow when possible, own when escape processing needed
}
```

Only works when you own the input buffer for the struct's lifetime. Streaming deserializers cannot borrow.

## serde_with — Extended Power

```rust
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Deserialize)]
struct Config {
    #[serde_as(as = "DisplayFromStr")]
    ip: IpAddr,                         // serialize via Display/FromStr

    #[serde_as(as = "[_; 3]")]
    fixed: [u8; 3],                     // const-generic arrays

    #[serde(skip_serializing_if = "Option::is_none")]
    optional: Option<String>,
}
```

## Library Authorship

- Make serde optional: `serde = { version = "1", optional = true }`
- Name the feature `serde` (not `with-serde`) per API Guidelines
- Chain features: `serde = ["dep:serde", "some-dep?/serde"]`
- Don't expose private state through derived impls

## Compile Time Impact

Serde's `derive` feature pulls in `syn` and proc-macro machinery — one of the largest compile-time costs. Mitigations:
- Gate behind optional feature in libraries
- Disable `derive` feature if only hand-writing impls
- Use `cargo build -Z timings` to profile

## Format Selection

| Format | Speed | Size | Human-readable | Cross-language |
|---|---|---|---|---|
| JSON (`serde_json`) | Moderate | Large | Yes | Yes |
| Bincode | Fast | Small | No | Rust-only |
| Postcard | Fast | Smallest | No | Limited |
| MessagePack (`rmp-serde`) | Fast | Small | No | Yes |
| TOML | Slow | Medium | Yes | Yes |

## Alternatives to Serde

| Crate | Use when |
|---|---|
| `rkyv` | True zero-copy (mmap/cast), Rust-only, latency-critical |
| `speedy` | Fastest binary serializer, Rust-to-Rust |
| `prost` | Protobuf interop with non-Rust services |
| `postcard` | Embedded / WASM / no_std with smallest output |
