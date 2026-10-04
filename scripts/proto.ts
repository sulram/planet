// The wire types, from the schemas into the tree. The core's, from proto/: Go
// under server/internal/protocol, Rust under crates/protocol/src/gen. A
// plugin's, from plugins/<name>/wire/: Rust under its world half's src/gen,
// and Go under server/internal/<name>/wire where its server half is a Go
// package (DECISIONS 99). All of it is committed, so building needs no
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

await $`buf lint`;
await $`buf generate proto --template proto/buf.gen.yaml`;
const made = ['server/internal/protocol', 'crates/protocol/src/gen'];

// A plugin's schema generates into the plugin's own halves, and names the
// core's types where the core's crate holds them.
for (const { name } of plugins()) {
	const wire = `plugins/${name}/wire`;
	if (!existsSync(`${ROOT}/${wire}`)) continue;
	const rust = `plugins/${name}/world/src/gen`;
	const outs = [{ local: 'protoc-gen-prost', out: rust, opt: ['extern_path=.planet.v1=::protocol::v1'] }];
	made.push(rust);
	if (existsSync(`${ROOT}/server/internal/${name}`)) {
		const go = `server/internal/${name}/wire`;
		outs.push({ local: 'protoc-gen-go', out: go, opt: [`module=github.com/sulram/planet/${go}`] });
		made.push(go);
	}
	const template = JSON.stringify({ version: 'v2', clean: true, plugins: outs });
	await $`buf generate ${wire} --template ${template}`;
}
console.log(`proto -> ${made.join(', ')}`);
