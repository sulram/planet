// What CI runs, locally, in the same order.
import { $ } from 'bun';
import { ROOT } from './lib';

$.cwd(ROOT);
await $`cargo fmt --all --check`;
await $`cargo clippy --workspace --all-targets -- -D warnings`;
await $`cargo test --workspace`;
await $`cargo build --release --target wasm32-unknown-unknown -p shell-web`;
await $`go vet ./...`.cwd(`${ROOT}/server`);
await $`go test ./...`.cwd(`${ROOT}/server`);
await $`bun run --cwd apps/web check`;
await $`bun test ./apps/web/src`;
// The wire types in the tree are the schema's: a stale generation is a
// schema change nobody built.
await $`bun scripts/proto.ts`;
await $`git diff --exit-code -- crates/protocol/src/gen server/internal/protocol`;
await $`bun scripts/docs.ts`;
console.log('all checks pass');
