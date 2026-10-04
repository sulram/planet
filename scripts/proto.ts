// The wire types, from proto/ into the tree. The core's: Go under
// server/internal/protocol, Rust under crates/protocol/src/gen. A plugin's,
// from proto/planet/<name>/: Go under server/internal/<name>/wire, Rust under
// crates/<name>/src/gen. All of it is committed, so building needs no
// generator; changing a schema does. The generators are pinned by setup.ts.
//   bun run proto
import { existsSync } from 'node:fs';
import { $ } from 'bun';
import { plugins, ROOT } from './lib';

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
await $`buf generate proto --template proto/buf.gen.yaml --path proto/planet/v1`;
const made = ['server/internal/protocol', 'crates/protocol/src/gen'];

// A plugin's schema generates into the plugin's own halves, and names the
// core's types where the core's crate holds them.
for (const { name } of plugins()) {
	if (!existsSync(`${ROOT}/proto/planet/${name}`)) continue;
	const go = `server/internal/${name}/wire`;
	const rust = `crates/${name}/src/gen`;
	const template = JSON.stringify({
		version: 'v2',
		clean: true,
		plugins: [
			{ local: 'protoc-gen-go', out: go, opt: [`module=github.com/sulram/planet/${go}`] },
			{ local: 'protoc-gen-prost', out: rust, opt: ['extern_path=.planet.v1=::protocol::v1'] }
		]
	});
	await $`buf generate proto --template ${template} --path proto/planet/${name}`;
	made.push(go, rust);
}
console.log(`proto -> ${made.join(', ')}`);
