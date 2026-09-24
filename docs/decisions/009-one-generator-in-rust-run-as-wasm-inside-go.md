# 09. One generator, in Rust, run as WASM inside Go (proposed)

Logged 2026-09-20.

The server and headless agents need terrain too. Compiling the Rust generator
to WASM and running it through wazero keeps a single source of truth without
CGO. Rejected: porting the generator to Go (two implementations will diverge).
