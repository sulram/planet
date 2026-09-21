// What a frame costs in WASM, against budgets. Fails when one is blown.
//   bun run bench
// Budgets are for the development machine (Apple silicon). A Raspberry Pi or
// a phone is several times slower, which is why they are this tight.
import { $ } from 'bun';
import { ROOT } from './lib';

const PATCH_SAMPLES = 35 * 35;
const PATCH_BUDGET_MS = 1.5;
const FRAME_BUDGET_MS = 12;
// A volume patch carries 50 times the samples of the height it replaces, so it
// gets its own budget: a handful of them a frame, not one a millisecond.
const VOLUME_BUDGET_MS = 8;

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

// A world shaped by a field is the other hot path, once one has been baked.
const field = Bun.file(`${ROOT}/assets/fields/earth.field`);
const runs: [number, string][] = [
	[0, 'planet'],
	[1, 'moon']
];
if (await field.exists()) {
	bench.load_field(await field.bytes());
	runs.push([2, 'planet over a field']);
} else {
	console.log('no field baked (bun run field): that path is not measured');
}

for (const [body, name] of runs) {
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

// What one body's collision costs, apart from the frame it lands in: a
// footing is what holds it up, and a step asks for up to four of them.
// A world full of agents is what the budget leaves room for: at this one,
// twenty of them walking cost a millisecond between them.
const FOOTING_BUDGET_MS = 0.05;
{
	const rounds = 2000;
	bench.footings(rounds);
	const start = performance.now();
	bench.footings(rounds);
	const ms = (performance.now() - start) / rounds;
	report('planet: four footings, one body one step', ms * 4, FOOTING_BUDGET_MS);
	console.log(`     ${(ms * 1000).toFixed(1)} us a footing`);
}

// A body running across cave country, where every frame reads the density
// field for what holds it up and for what stops it. Terrain keeps streaming
// under it, so this is the whole frame and not only the collision in it.
{
	bench.walk_start();
	let worst = 0;
	for (let frame = 0; frame < 600; frame++) {
		const before = performance.now();
		bench.walk_frame();
		worst = Math.max(worst, performance.now() - before);
	}
	report('planet: worst frame of a 10 s walk in cave country', worst, FRAME_BUDGET_MS);
}

// One patch of the finest quadtree level, as a volume instead of a height:
// 32 x 32 columns and a 32 m window of cells up each. What this costs against
// `samples` is what says whether the volume layer can be meshed in a frame.
const VOLUME_COLUMNS = 32 * 32;
const VOLUME_CELLS = 64;
{
	const rounds = 4;
	bench.volume(rounds, VOLUME_COLUMNS, VOLUME_CELLS);
	const start = performance.now();
	bench.volume(rounds, VOLUME_COLUMNS, VOLUME_CELLS);
	const ms = (performance.now() - start) / rounds;
	report('planet: one patch as a volume', ms, VOLUME_BUDGET_MS);
	console.log(
		`     ${(VOLUME_COLUMNS * VOLUME_CELLS).toLocaleString()} cells, ` +
			`${((ms * 1e6) / (VOLUME_COLUMNS * VOLUME_CELLS)).toFixed(0)} ns a cell`
	);
}

if (failed) process.exit(1);
