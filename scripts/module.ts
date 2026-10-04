// Builds the server's module: crates/module -> WASM -> the file the Go server
// embeds (DECISIONS 102). The Go server builds and tests only after this.
//   bun run module
import { copyFileSync } from 'node:fs';
import { $ } from 'bun';
import { ROOT } from './lib';

const WASM = `${ROOT}/target/wasm32-unknown-unknown/release/module.wasm`;
const OUT = `${ROOT}/server/internal/module/world.wasm`;

export async function buildModule(): Promise<void> {
	$.cwd(ROOT);
	await $`cargo build --release --target wasm32-unknown-unknown -p module`;
	copyFileSync(WASM, OUT);
	console.log(`module -> ${OUT}`);
}

if (import.meta.main) await buildModule();
