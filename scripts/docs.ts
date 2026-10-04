// What a script can check about the docs. `bun run docs` fails when a doc and
// the repo disagree (runs in `bun run check` and in CI); `bun run docs gen`
// rewrites what is generated (the DECISIONS index). The rules it holds are in
// CLAUDE.md § Docs style and § How the engine stays reusable.
import { existsSync, readdirSync, readFileSync, readlinkSync, writeFileSync } from 'node:fs';
import { ROOT } from './lib';

const mode = process.argv[2] === 'gen' ? 'gen' : 'check';
const problems: string[] = [];
const read = (p: string) => readFileSync(`${ROOT}/${p}`, 'utf8');
const docs = readdirSync(`${ROOT}/docs`)
	.filter((f) => f.endsWith('.md'))
	.map((f) => `docs/${f}`);
const claude = read('CLAUDE.md');

/* --------------------------------- the decisions log: files, and their index */

type Entry = { n: number; file: string; title: string; status: string; date: string; retires: number[]; removes: number[] };
const entriesByFile: Entry[] = [];
for (const f of readdirSync(`${ROOT}/docs/decisions`).filter((f) => f.endsWith('.md')).sort()) {
	const text = read(`docs/decisions/${f}`);
	const head = text.match(/^# (~~)?(\d+)\. (.*?)(~~)? \((.*?)\)\s*$/m);
	if (!head) {
		problems.push(`docs/decisions/${f}: the first line is not "# NN. Title (status)"`);
		continue;
	}
	const struck = Boolean(head[1]);
	const retires = [...text.matchAll(/\bretires (\d+)\b/g)].map((m) => Number(m[1]));
	// `**Removes:** 11, 13.` names entries whose subject left the project: their files go.
	const removes = (text.match(/^\*\*Removes:\*\* ([\d, ]+)\./m)?.[1] ?? '').split(',').map((s) => Number(s.trim())).filter(Boolean);
	entriesByFile.push({
		n: Number(head[2]),
		file: f,
		title: head[3],
		status: struck ? `retired, ${head[5]}` : head[5],
		date: text.match(/^Logged (\d{4}-\d{2}-\d{2})\./m)?.[1] ?? '',
		retires,
		removes,
	});
	if (!f.startsWith(head[2].padStart(3, '0') + '-')) problems.push(`docs/decisions/${f}: the file name does not start with ${head[2].padStart(3, '0')}-`);
	if (!/^# .*\n\nLogged \d{4}-\d{2}-\d{2}\./.test(text)) problems.push(`docs/decisions/${f}: the second paragraph is not "Logged YYYY-MM-DD."`);
}
const entries = new Set(entriesByFile.map((e) => e.n));
// A removed entry is gone, file and all, and its number stays taken: by the entry that removed it.
const removed = new Map<number, number>();
for (const e of entriesByFile) for (const r of e.removes) removed.set(r, e.n);
entriesByFile.sort((a, b) => a.n - b.n);
for (const [i, e] of entriesByFile.entries()) {
	const dup = entriesByFile.filter((x) => x.n === e.n);
	if (dup.length > 1 && dup[0] === e) problems.push(`docs/decisions: number ${e.n} is claimed by ${dup.map((x) => x.file).join(' and ')}`);
	if (i > 0) for (let n = entriesByFile[i - 1].n + 1; n < e.n; n++) if (!removed.has(n)) problems.push(`docs/decisions: number ${n} is skipped before ${e.file}`);
	if (removed.has(e.n)) problems.push(`docs/decisions/${e.file}: removed by ${removed.get(e.n)}, and the file is still here`);
	for (const r of e.retires) {
		const old = entriesByFile.find((x) => x.n === r);
		if (old && !old.status.startsWith('retired')) problems.push(`docs/decisions/${old.file}: retired by ${e.n}, and its title is not struck`);
	}
}

const index = ['| # | Logged | Decision | Status |', '|---|---|---|---|', ...entriesByFile.map((e) => `| ${String(e.n).padStart(2, '0')} | ${e.date} | [${e.title}](decisions/${e.file}) | ${e.status} |`)].join('\n');
{
	const file = 'docs/DECISIONS.md';
	const text = read(file);
	const re = /(<!-- generated:index -->)[\s\S]*?(<!-- \/generated:index -->)/;
	if (!re.test(text)) problems.push(`${file}: the <!-- generated:index --> markers are missing`);
	else {
		const next = text.replace(re, (_, open, close) => `${open}\n${index}\n${close}`);
		if (next !== text) {
			if (mode === 'gen') writeFileSync(`${ROOT}/${file}`, next);
			else problems.push(`${file}: the index is stale (run bun run docs gen)`);
		}
	}
}

/* ------------------------------------------------- DECISIONS N names an entry */

const sources = new Bun.Glob('{docs/**/*.md,CLAUDE.md,apps/web/CLAUDE.md,server/CLAUDE.md,README.md,crates/**/*.{rs,wgsl},plugins/**/*.{rs,ts,svelte,proto},proto/**/*.proto,server/**/*.go,apps/web/src/**/*.{ts,svelte},scripts/*.ts}');
const files = [...sources.scanSync(ROOT)].filter((f) => !f.includes('node_modules') && !f.includes('/target/'));
for (const file of files) {
	read(file)
		.split('\n')
		.forEach((line, i) => {
			for (const m of line.matchAll(/DECISIONS ?(\d+)/g)) {
				const n = Number(m[1]);
				if (entries.has(n)) continue;
				// An entry is written once, so it may still name one removed since.
				if (removed.has(n) && file.startsWith('docs/decisions/')) continue;
				problems.push(removed.has(n) ? `${file}:${i + 1}: DECISIONS ${n} was removed by ${removed.get(n)}` : `${file}:${i + 1}: DECISIONS ${n} does not exist`);
			}
		});
}

/* ---------------------------- every CLAUDE.md has an AGENTS.md symlink beside it */

for (const guide of new Bun.Glob('**/CLAUDE.md').scanSync({ cwd: ROOT, dot: false })) {
	if (guide.includes('node_modules') || guide.startsWith('refs/') || guide.includes('/target/')) continue;
	const dir = guide.slice(0, -'CLAUDE.md'.length);
	const agents = `${ROOT}/${dir}AGENTS.md`;
	let target: string | null = null;
	try {
		target = readlinkSync(agents);
	} catch {
		problems.push(`${dir}AGENTS.md: missing; it is a symlink to CLAUDE.md (ln -s CLAUDE.md ${dir}AGENTS.md)`);
		continue;
	}
	if (target !== 'CLAUDE.md') problems.push(`${dir}AGENTS.md: points at ${target}; it is a symlink to CLAUDE.md`);
}

/* ------------------------------------ dependency direction is law (CLAUDE.md) */

// What each core crate may import of ours. Shells and bench are leaves and
// may import anything. A new arrow is a decision: log it, then add it here.
const ALLOWED: Record<string, string[]> = {
	topology: [],
	scene: [],
	protocol: [],
	voxel: [],
	seat: ['topology', 'worldgen', 'voxel', 'protocol'],
	worldgen: ['topology'],
	avatar: ['scene'],
	render: ['scene'],
	client: ['topology', 'worldgen', 'scene', 'avatar', 'voxel', 'seat', 'protocol'],
	'ui-native': ['client', 'scene'],
	// What a plugin's world half stands on, and the host of world halves. It
	// never imports `client`, and neither does the module that is its shell.
	world: ['topology', 'protocol', 'worldgen', 'voxel', 'seat'],
	module: ['world', 'plugins-world', 'protocol', 'worldgen'],
	// A plugin's rows name crates of the core and its own world half, never
	// another plugin's (DECISIONS 93). A world half's names no `client`.
	'chat-world': ['world', 'protocol'],
	'chat-client': ['client', 'protocol', 'chat-world'],
	'build-world': ['world', 'voxel'],
	'build-client': ['client', 'topology', 'voxel', 'build-world'],
};
// A crate is a folder with a manifest: the core's under crates/, and a
// plugin's two under plugins/<name>/, `<name>-client` and `<name>-world`
// (DECISIONS 100, 101).
const cratesIn = (dir: string) => readdirSync(`${ROOT}/${dir}`).filter((d) => existsSync(`${ROOT}/${dir}/${d}/Cargo.toml`));
const HALVES = ['client', 'world'];
const folders = readdirSync(`${ROOT}/plugins`).filter((d) => HALVES.some((half) => existsSync(`${ROOT}/plugins/${d}/${half}/Cargo.toml`)));
// Where each plugin crate's manifest is, and which plugin it belongs to.
const plugged = new Map<string, { path: string; plugin: string }>();
for (const plugin of folders) {
	for (const half of HALVES) {
		const path = `plugins/${plugin}/${half}`;
		if (!existsSync(`${ROOT}/${path}/Cargo.toml`)) continue;
		const name = read(`${path}/Cargo.toml`).match(/^name = "([^"]+)"/m)?.[1];
		if (name !== `${plugin}-${half}`) problems.push(`${path}/Cargo.toml: the crate is named \`${plugin}-${half}\``);
		plugged.set(`${plugin}-${half}`, { path, plugin });
	}
}
const ours = [...cratesIn('crates'), ...plugged.keys()];
for (const [crate, { path, plugin }] of plugged) {
	if (!(crate in ALLOWED)) problems.push(`${path}: no row in ALLOWED (scripts/docs.ts) says what it may import`);
	for (const other of ALLOWED[crate] ?? []) {
		if (plugged.has(other) && plugged.get(other)?.plugin !== plugin) problems.push(`${path}: its row in ALLOWED names \`${other}\`, another plugin's (DECISIONS 93)`);
		if (crate.endsWith('-world') && other === 'client') problems.push(`${path}: a world half never imports \`client\` (DECISIONS 99)`);
	}
}
for (const [crate, allowed] of Object.entries(ALLOWED)) {
	if (!ours.includes(crate)) continue;
	const home = plugged.get(crate)?.path ?? `crates/${crate}`;
	const toml = read(`${home}/Cargo.toml`);
	const deps = toml.slice(toml.indexOf('[dependencies]'), toml.indexOf('[dev-dependencies]') === -1 ? undefined : toml.indexOf('[dev-dependencies]'));
	for (const m of deps.matchAll(/^([a-z-]+)(?:\.workspace| *=)/gm)) {
		if (ours.includes(m[1]) && !allowed.includes(m[1])) problems.push(`${home}: imports \`${m[1]}\`, which CLAUDE.md § How it grows forbids`);
	}
}

/* ------------------------------- `unsafe` is written in one file (DECISIONS 102) */

// The workspace forbids `unsafe`. The module says the three names the server
// knows it by, and they are the exception: one file, three marks, in a crate
// that denies the rest. Every other crate inherits the workspace's lints.
const ABI = 'crates/module/src/abi.rs';
const MARKS = 3;
for (const file of new Bun.Glob('{crates,plugins}/**/*.rs').scanSync(ROOT)) {
	if (file.includes('node_modules') || file.includes('/target/')) continue;
	const marks = read(file).match(/allow\(unsafe_code\)/g)?.length ?? 0;
	if (file === ABI ? marks !== MARKS : marks !== 0) problems.push(`${file}: ${marks} allow(unsafe_code); ${ABI} holds ${MARKS} and no other file any`);
}
for (const crate of ours) {
	const home = plugged.get(crate)?.path ?? `crates/${crate}`;
	const toml = read(`${home}/Cargo.toml`);
	if (crate === 'module') {
		if (!/^\[lints\.rust\]\nunsafe_code = "deny"$/m.test(toml)) problems.push(`${home}/Cargo.toml: the module denies unsafe_code, and allows it on its three names alone`);
	} else if (!/^\[lints\]\nworkspace = true$/m.test(toml)) problems.push(`${home}/Cargo.toml: no \`[lints] workspace = true\`, so \`unsafe\` is not forbidden there`);
}

/* ------------------------------------------- crate bullets match crates/ */

const layout = claude.slice(claude.indexOf('## Layout'), claude.indexOf('## Invariants'));
const crates = cratesIn('crates');
for (const c of crates) if (!layout.includes(`\`${c}\``)) problems.push(`CLAUDE.md § Layout: crate \`${c}\` has no bullet`);
for (const m of layout.matchAll(/^ {2}- `([a-z-]+)`/gm)) {
	if (!crates.includes(m[1])) problems.push(`CLAUDE.md § Layout: \`${m[1]}\` is not a crate`);
}
for (const p of folders) if (!layout.includes(`\`${p}\``)) problems.push(`CLAUDE.md § Layout: plugin \`${p}\` is not named`);

/* -------------------------------------- bun run names match package.json */

const scripts = Object.keys(JSON.parse(read('package.json')).scripts as Record<string, string>);
const listed = [...layout.matchAll(/`bun run ([^`]+)`/g)].map((m) => m[1]).find((l) => l.includes('|'))?.split('|').map((s) => s.trim()) ?? [];
for (const s of scripts) if (!listed.includes(s)) problems.push(`CLAUDE.md § Layout: script \`${s}\` is not in the bun run list`);
for (const s of listed) if (!scripts.includes(s)) problems.push(`CLAUDE.md § Layout: \`bun run ${s}\` is not a script`);
for (const file of [...docs, 'CLAUDE.md', 'apps/web/CLAUDE.md', 'server/CLAUDE.md', 'README.md']) {
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

for (const file of [...docs, 'CLAUDE.md']) {
	if (file.endsWith('DECISIONS.md') || file.endsWith('VISION.md')) continue;
	const lines = read(file).trimEnd().split('\n').length;
	if (lines > 200) problems.push(`${file}: ${lines} lines; over 200 it splits by theme`);
}

if (problems.length) {
	console.error(problems.map((p) => `x ${p}`).join('\n'));
	console.error(`\n${problems.length} problem(s) in the docs`);
	process.exit(1);
}
console.log(mode === 'gen' ? 'docs generated and checked' : 'docs agree with the repo');
