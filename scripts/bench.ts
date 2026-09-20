// What a frame costs in WASM, against budgets. Fails when one is blown.
//   bun run bench
// Budgets are for the development machine (Apple silicon). A Raspberry Pi or
// a phone is several times slower, which is why they are this tight.
import { $ } from 'bun';
import { ROOT } from './lib';

const PATCH_SAMPLES = 35 * 35;
const PATCH_BUDGET_MS = 1.5;
const FRAME_BUDGET_MS = 12;

$.cwd(ROOT);
const OUT = `${ROOT}/target/bench-pkg`;
await $`cargo build --release --target wasm32-unknown-unknown -p bench`;
await $`wasm-bindgen --target web --out-dir ${OUT} ${ROOT}/target/wasm32-unknown-unknown/release/bench.wasm`;
const bench = await import(`${OUT}/bench.js`);
bench.initSync({ module: await Bun.file(`${OUT}/bench_bg.wasm`).bytes() });

let failed = false;
const report = (what: string, ms: number, budget: number) => {
	const over = ms > budget;
	failed ||= over;
	console.log(`${over ? 'OVER' : 'ok  '} ${what}: ${ms.toFixed(2)} ms (budget ${budget})`);
};

for (const [body, name] of [[0, 'planet'], [1, 'moon']] as const) {
	const rounds = 40;
	bench.samples(body, PATCH_SAMPLES * rounds);
	const start = performance.now();
	bench.samples(body, PATCH_SAMPLES * rounds);
	report(`${name}: samples of one patch`, (performance.now() - start) / rounds, PATCH_BUDGET_MS);

	bench.descent_start(body);
	let worst = 0;
	for (let frame = 0; frame < 600; frame++) {
		const before = performance.now();
		bench.descent_frame();
		worst = Math.max(worst, performance.now() - before);
	}
	report(`${name}: worst frame of a 10 s descent`, worst, FRAME_BUDGET_MS);
}
if (failed) process.exit(1);
