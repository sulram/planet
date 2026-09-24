// What a script can check about the docs. `bun run docs` fails when a doc and
// the repo disagree (runs in `bun run check` and in CI). The rules it holds
// are in CLAUDE.md § Docs style.
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { ROOT } from './lib';

const problems: string[] = [];
const read = (p: string) => readFileSync(`${ROOT}/${p}`, 'utf8');
const docs = readdirSync(`${ROOT}/docs`)
	.filter((f) => f.endsWith('.md'))
	.map((f) => `docs/${f}`);
const claude = read('CLAUDE.md');

/* ------------------------------------------------- DECISIONS N names an entry */

const entries = new Set([...read('docs/DECISIONS.md').matchAll(/^## ~*(\d+)\./gm)].map((m) => Number(m[1])));
const sources = new Bun.Glob('{docs/*.md,CLAUDE.md,README.md,crates/**/*.{rs,wgsl},server/**/*.go,apps/web/src/**/*.{ts,svelte},scripts/*.ts}');
const files = [...sources.scanSync(ROOT)].filter((f) => !f.includes('node_modules') && !f.includes('/target/'));
for (const file of files) {
	read(file)
		.split('\n')
		.forEach((line, i) => {
			for (const m of line.matchAll(/DECISIONS ?(\d+)/g)) {
				if (!entries.has(Number(m[1]))) problems.push(`${file}:${i + 1}: DECISIONS ${m[1]} does not exist`);
			}
		});
}

/* ------------------------------------ dependency direction is law (CLAUDE.md) */

// What each core crate may import of ours. Shells and bench are leaves and
// may import anything. A new arrow is a decision: log it, then add it here.
const ALLOWED: Record<string, string[]> = {
	topology: [],
	scene: [],
	protocol: [],
	worldgen: ['topology'],
	avatar: ['scene'],
	render: ['scene'],
	client: ['topology', 'worldgen', 'scene', 'avatar', 'protocol'],
	'ui-native': ['client', 'scene'],
};
const ours = readdirSync(`${ROOT}/crates`).filter((d) => existsSync(`${ROOT}/crates/${d}/Cargo.toml`));
for (const [crate, allowed] of Object.entries(ALLOWED)) {
	if (!ours.includes(crate)) continue;
	const toml = read(`crates/${crate}/Cargo.toml`);
	const deps = toml.slice(toml.indexOf('[dependencies]'), toml.indexOf('[dev-dependencies]') === -1 ? undefined : toml.indexOf('[dev-dependencies]'));
	for (const m of deps.matchAll(/^([a-z-]+)(?:\.workspace| *=)/gm)) {
		if (ours.includes(m[1]) && !allowed.includes(m[1])) problems.push(`crates/${crate}: imports \`${m[1]}\`, which CLAUDE.md § How the engine stays reusable forbids`);
	}
}

/* ------------------------------------------- crate bullets match crates/ */

const layout = claude.slice(claude.indexOf('## Layout'), claude.indexOf('## Invariants'));
const crates = readdirSync(`${ROOT}/crates`).filter((d) => existsSync(`${ROOT}/crates/${d}/Cargo.toml`));
for (const c of crates) if (!layout.includes(`\`${c}\``)) problems.push(`CLAUDE.md § Layout: crate \`${c}\` has no bullet`);
for (const m of layout.matchAll(/^ {2}- `([a-z-]+)`/gm)) {
	if (!crates.includes(m[1])) problems.push(`CLAUDE.md § Layout: \`${m[1]}\` is not a crate`);
}

/* -------------------------------------- bun run names match package.json */

const scripts = Object.keys(JSON.parse(read('package.json')).scripts as Record<string, string>);
const listed = [...layout.matchAll(/`bun run ([^`]+)`/g)].map((m) => m[1]).find((l) => l.includes('|'))?.split('|').map((s) => s.trim()) ?? [];
for (const s of scripts) if (!listed.includes(s)) problems.push(`CLAUDE.md § Layout: script \`${s}\` is not in the bun run list`);
for (const s of listed) if (!scripts.includes(s)) problems.push(`CLAUDE.md § Layout: \`bun run ${s}\` is not a script`);
for (const file of [...docs, 'CLAUDE.md', 'README.md']) {
	for (const m of read(file).matchAll(/bun run ([a-z]+)/g)) {
		if (!scripts.includes(m[1])) problems.push(`${file}: \`bun run ${m[1]}\` is not a script`);
	}
}

/* ------------------------------------------------------------ no em dash */

for (const file of files) {
	read(file)
		.split('\n')
		.forEach((line, i) => {
			if (line.includes('—')) problems.push(`${file}:${i + 1}: em dash`);
		});
}

/* ------------------------------------------- an operational doc splits at 200 */

for (const file of docs) {
	if (file.endsWith('DECISIONS.md') || file.endsWith('VISION.md')) continue;
	const lines = read(file).trimEnd().split('\n').length;
	if (lines > 200) problems.push(`${file}: ${lines} lines; over 200 it splits by theme`);
}

if (problems.length) {
	console.error(problems.map((p) => `x ${p}`).join('\n'));
	console.error(`\n${problems.length} problem(s) in the docs`);
	process.exit(1);
}
console.log('docs agree with the repo');
