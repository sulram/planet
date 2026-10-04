// One-time setup after clone: dependencies, git hooks, Rust targets and tools.
import { $ } from 'bun';
import { ROOT } from './lib';

$.cwd(ROOT);

await $`bun install`;
await $`git config core.hooksPath scripts/hooks`;
await $`rustup target add wasm32-unknown-unknown`;

// wasm-bindgen-cli must match the `wasm-bindgen` crate version in Cargo.lock.
const lock = await Bun.file(`${ROOT}/Cargo.lock`).text();
const version = lock.match(/name = "wasm-bindgen"\nversion = "([^"]+)"/)?.[1];
if (!version) throw new Error('wasm-bindgen not found in Cargo.lock');
const have = await $`wasm-bindgen --version`.nothrow().quiet().text();
if (!have.includes(version)) {
	console.log(`installing wasm-bindgen-cli ${version}`);
	await $`cargo install wasm-bindgen-cli --version ${version} --locked`;
}

// The protocol generators, pinned: buf drives protoc-gen-go (the version
// go.mod pins) and protoc-gen-prost (built for the prost in Cargo.lock).
const BUF = 'v1.73.0';
const PROTOC_GEN_PROST = '0.5.0';
// `go install` puts binaries in GOBIN, or under GOPATH when it is unset.
const gobin = (await $`go env GOBIN`.text()).trim() || (await $`go env GOPATH`.text()).trim() + '/bin';
const goTool = async (name: string, module: string, version: string) => {
	const have = await $`${gobin}/${name} --version`.nothrow().quiet().text();
	if (!have.includes(version.replace(/^v/, ''))) {
		console.log(`installing ${name} ${version}`);
		await $`go install ${module}@${version}`;
	}
};
await goTool('buf', 'github.com/bufbuild/buf/cmd/buf', BUF);
const gomod = await Bun.file(`${ROOT}/server/go.mod`).text();
const protobuf = gomod.match(/google\.golang\.org\/protobuf (v[^\s]+)/)?.[1];
if (!protobuf) throw new Error('google.golang.org/protobuf not found in server/go.mod');
await goTool('protoc-gen-go', 'google.golang.org/protobuf/cmd/protoc-gen-go', protobuf);
const prostGen = await $`protoc-gen-prost --version`.nothrow().quiet().text();
if (!prostGen.includes(PROTOC_GEN_PROST)) {
	console.log(`installing protoc-gen-prost ${PROTOC_GEN_PROST}`);
	await $`cargo install protoc-gen-prost --version ${PROTOC_GEN_PROST} --locked`;
}

console.log('setup done');
