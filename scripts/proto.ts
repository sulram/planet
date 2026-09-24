// The wire types, from proto/ into the tree: Go under server/internal/protocol,
// Rust under crates/protocol/src/gen. Both are committed, so building needs no
// generator; changing the schema does. The generators are pinned by setup.ts.
//   bun run proto
import { $ } from 'bun';
import { ROOT } from './lib';

$.cwd(ROOT);
// `go install` puts binaries in GOBIN, or under GOPATH when it is unset.
const gobin = (await $`go env GOBIN`.text()).trim() || (await $`go env GOPATH`.text()).trim() + '/bin';
$.env({ ...process.env, PATH: `${gobin}:${process.env.PATH}` });

for (const tool of ['buf', 'protoc-gen-go', 'protoc-gen-prost']) {
	if (!Bun.which(tool, { PATH: `${gobin}:${process.env.PATH}` })) {
		throw new Error(`${tool} not found: run \`bun run setup\``);
	}
}

await $`buf lint proto`;
await $`buf generate proto --template proto/buf.gen.yaml`;
console.log('proto -> server/internal/protocol, crates/protocol/src/gen');
