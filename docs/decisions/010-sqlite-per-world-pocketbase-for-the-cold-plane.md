# 10. SQLite per world, PocketBase for the cold plane (decided; split proposed)

Logged 2026-09-20.

Marlus chose SQLite and PocketBase, which he already runs in production. The
split is ours: PocketBase holds accounts, worlds, volumes, roles and records;
chunks and the op log live in a separate `world.db` driven by our code, because
voxel edits are frequent, binary and tiny, and PocketBase realtime is SSE +
JSON with no Rust SDK. PocketBase is pre-1.0, so the core never imports it.
