// Builds the browser client: shell-web -> WASM -> apps/web/src/lib/engine/pkg.
import { $ } from 'bun';
import { ROOT } from './lib';

const OUT = `${ROOT}/apps/web/src/lib/engine/pkg`;
const WASM = `${ROOT}/target/wasm32-unknown-unknown/release/shell_web.wasm`;

export async function buildWasm(): Promise<void> {
	if (!Bun.which('wasm-bindgen')) throw new Error('wasm-bindgen not found: run `bun run setup`');
	$.cwd(ROOT);
	await $`cargo build --release --target wasm32-unknown-unknown -p shell-web`;
	await $`wasm-bindgen --target web --out-dir ${OUT} ${WASM}`;
	console.log(`engine -> ${OUT}`);
}

if (import.meta.main) await buildWasm();
