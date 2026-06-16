# Haul — Agent / AI Coding Guidelines

## Project

P2P encrypted file rooms built on iroh (v1.0), iroh-blobs, iroh-docs, iroh-gossip.
Rust workspace: `crates/haul-core` (library) + `crates/haul-cli` (CLI).
License: MPL-2.0. Future: UniFFI bindings for mobile/desktop.

## Code style

- No magic strings — all constants in `constants.rs`, key-prefix helpers in `keys.rs`
- No `unwrap()` in library code — propagate errors with `anyhow::Result`
- No unnecessary comments — name things so they're self-documenting
- No backwards-compat shims — delete unused code
- Short, flat modules — prefer editing existing files over creating new ones

## Dependencies

- Crypto: `chacha20poly1305` (ChaCha20-Poly1305). Do NOT switch to AES-GCM
- Serialization: `postcard` for wire (tickets), `serde_json` for disk (rooms.json, doc entries)
- Async: tokio full. Use `tokio::pin!` when streaming iroh-docs entries
- Error handling: `anyhow` everywhere

## Testing philosophy

**Test logic, not plumbing.**

- Unit tests live in the same file as the code under test (`#[cfg(test)]` module at bottom)
- Test pure functions directly — crypto round-trips, key encoding, ticket encode/decode
- Do NOT mock iroh or the network — that tests nothing real
- Do NOT write tests that just spin up `HaulNode` and call the full stack — integration tests come later
- Prefer many small focused tests over one big test that checks everything
- Name tests after the invariant: `encrypt_decrypt_roundtrip`, `wrong_key_fails`, `ticket_roundtrip`

## iroh API notes (v1.0 / iroh-blobs 0.103 / iroh-docs 0.101)

- `Gossip::builder().spawn(endpoint)` is NOT async — no `.await`
- `Docs::persistent(path).spawn(endpoint, blobs.into(), gossip)` — blobs BEFORE gossip; blobs must be `iroh_blobs::api::Store` (use `fs_store.clone().into()`)
- `iroh::EndpointId` is the node identity type (NOT `iroh::endpoint::NodeId`)
- Doc entry content retrieved via `node.blobs.blobs().get_bytes(entry.content_hash())`
- `ShareMode` / `AddrInfoOptions` live at `iroh_docs::api::protocol::{ShareMode, AddrInfoOptions}`
- Stream from `doc.get_many()` requires `tokio::pin!` before iterating with `StreamExt::next()`
