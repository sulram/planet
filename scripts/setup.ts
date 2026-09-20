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

if (!Bun.which('mailpit')) {
	console.warn('mailpit not found: dev emails print to the server log only (brew install mailpit)');
}
console.log('setup done');
