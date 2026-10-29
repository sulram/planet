// The checks only this repo needs, run by `kb check` after kb's own
// (kb.config.ts). The rules they hold are in AGENTS.md § Docs and § How it
// grows, and docs/LAYOUT.md; a rule a script can hold lives here, never in a
// reader's memory.
import type { Problem } from '@tekne/kb/config';
import { existsSync, readdirSync, readFileSync } from 'node:fs';

const err = (where: string, message: string): Problem => ({ level: 'error', where, message });

export const checks = [planet];

function planet(root: string): Problem[] {
	const out: Problem[] = [];
	const read = (p: string) => readFileSync(`${root}/${p}`, 'utf8');
	const problem = (where: string, message: string) => out.push(err(where, message));
	const docs = readdirSync(`${root}/docs`)
		.filter((f) => f.endsWith('.md'))
		.map((f) => `docs/${f}`);
	// Where the project's text lives: docs, guides, code and schemas.
	const sources = new Bun.Glob(
		'{docs/**/*.md,AGENTS.md,apps/web/AGENTS.md,server/AGENTS.md,README.md,crates/**/*.{rs,wgsl},plugins/**/*.{rs,ts,svelte,proto},proto/**/*.proto,server/**/*.go,apps/web/src/**/*.{ts,svelte},scripts/*.ts}'
	);
	const files = [...sources.scanSync(root)].filter((f) => !f.includes('node_modules') && !f.includes('/target/'));

	/* ------------------------------- DECISIONS <number or slug> names one entry */

	// A decision is cited by a fragment of its id: the number the old entries
	// carry (`DECISIONS 81`) or a piece of the slug (`DECISIONS figure-eases`).
	// The fragment names exactly one file. An entry is written once, so one
	// may still name an entry removed since (`**Removes:** 11, 13.`).
	const ids = readdirSync(`${root}/docs/decisions`)
		.filter((f) => f.endsWith('.md') && f !== 'index.md')
		.map((f) => f.slice(0, -3));
	const removed = new Set<string>();
	for (const id of ids) {
		const line = read(`docs/decisions/${id}.md`).match(/^\*\*Removes:\*\* ([\d, ]+)\./m)?.[1] ?? '';
		for (const n of line.split(',').map((s) => s.trim()).filter(Boolean)) removed.add(n);
	}
	const named = (fragment: string) => ids.filter((id) => (/^\d+$/.test(fragment) ? id.includes(`-${fragment.padStart(3, '0')}-`) : id.includes(fragment)));
	for (const file of files) {
		read(file)
			.split('\n')
			.forEach((line, i) => {
				for (const m of line.matchAll(/DECISIONS ?(\d+|[a-z0-9]+(?:-[a-z0-9]+)+)/g)) {
					const hits = named(m[1]);
					if (hits.length === 1) continue;
					if (hits.length > 1) problem(`${file}:${i + 1}`, `DECISIONS ${m[1]} names ${hits.length} entries: ${hits.join(', ')}`);
					else if (removed.has(m[1]) && !file.startsWith('docs/decisions/')) problem(`${file}:${i + 1}`, `DECISIONS ${m[1]} was removed`);
					else if (!removed.has(m[1])) problem(`${file}:${i + 1}`, `DECISIONS ${m[1]} does not exist`);
				}
			});
	}

	/* ------------------------------------ dependency direction is law (AGENTS.md) */

	// What each core crate may import of ours. Shells and bench are leaves and
	// may import anything. A new arrow is a decision: log it, then add it here.
	const ALLOWED: Record<string, string[]> = {
		topology: [],
		scene: [],
		protocol: [],
		voxel: [],
		part: [],
		vox: [],
		seat: ['topology', 'worldgen', 'voxel', 'part', 'protocol'],
		worldgen: ['topology'],
		avatar: ['scene'],
		render: ['scene'],
		client: ['topology', 'worldgen', 'scene', 'avatar', 'voxel', 'vox', 'part', 'seat', 'protocol'],
		'ui-native': ['client', 'scene'],
		// What a plugin's world half stands on, and the host of world halves. It
		// never imports `client`, and neither does the module that is its shell.
		world: ['topology', 'protocol', 'worldgen', 'voxel', 'part', 'seat'],
		module: ['world', 'plugins-world', 'protocol', 'worldgen'],
		// A plugin's rows name crates of the core and its own world half, never
		// another plugin's (DECISIONS 93). A world half's names no `client`.
		'chat-world': ['world', 'protocol'],
		'chat-client': ['client', 'protocol', 'chat-world'],
		'build-world': ['world', 'voxel'],
		'build-client': ['client', 'topology', 'voxel', 'build-world'],
		'items-world': ['world'],
		'items-client': ['client', 'topology', 'voxel', 'part', 'items-world']
	};
	// A crate is a folder with a manifest: the core's under crates/, and a
	// plugin's two under plugins/<name>/, `<name>-client` and `<name>-world`
	// (DECISIONS 100, 101).
	const cratesIn = (dir: string) => readdirSync(`${root}/${dir}`).filter((d) => existsSync(`${root}/${dir}/${d}/Cargo.toml`));
	const HALVES = ['client', 'world'];
	const folders = readdirSync(`${root}/plugins`).filter((d) => HALVES.some((half) => existsSync(`${root}/plugins/${d}/${half}/Cargo.toml`)));
	// Where each plugin crate's manifest is, and which plugin it belongs to.
	const plugged = new Map<string, { path: string; plugin: string }>();
	for (const plugin of folders) {
		for (const half of HALVES) {
			const path = `plugins/${plugin}/${half}`;
			if (!existsSync(`${root}/${path}/Cargo.toml`)) continue;
			const name = read(`${path}/Cargo.toml`).match(/^name = "([^"]+)"/m)?.[1];
			if (name !== `${plugin}-${half}`) problem(`${path}/Cargo.toml`, `the crate is named \`${plugin}-${half}\``);
			plugged.set(`${plugin}-${half}`, { path, plugin });
		}
	}
	const ours = [...cratesIn('crates'), ...plugged.keys()];
	for (const [crate, { path, plugin }] of plugged) {
		if (!(crate in ALLOWED)) problem(path, 'no row in ALLOWED (scripts/kb-plugins.ts) says what it may import');
		for (const other of ALLOWED[crate] ?? []) {
			if (plugged.has(other) && plugged.get(other)?.plugin !== plugin) problem(path, `its row in ALLOWED names \`${other}\`, another plugin's (DECISIONS 93)`);
			if (crate.endsWith('-world') && other === 'client') problem(path, 'a world half never imports `client` (DECISIONS 99)');
		}
	}
	for (const [crate, allowed] of Object.entries(ALLOWED)) {
		if (!ours.includes(crate)) continue;
		const home = plugged.get(crate)?.path ?? `crates/${crate}`;
		const toml = read(`${home}/Cargo.toml`);
		const deps = toml.slice(toml.indexOf('[dependencies]'), toml.indexOf('[dev-dependencies]') === -1 ? undefined : toml.indexOf('[dev-dependencies]'));
		for (const m of deps.matchAll(/^([a-z-]+)(?:\.workspace| *=)/gm)) {
			if (ours.includes(m[1]) && !allowed.includes(m[1])) problem(home, `imports \`${m[1]}\`, which AGENTS.md § How it grows forbids`);
		}
	}

	/* ------------------------------- `unsafe` is written in one file (DECISIONS 102) */

	// The workspace forbids `unsafe`. The module says the three names the server
	// knows it by, and they are the exception: one file, three marks, in a crate
	// that denies the rest. Every other crate inherits the workspace's lints.
	const ABI = 'crates/module/src/abi.rs';
	const MARKS = 3;
	for (const file of new Bun.Glob('{crates,plugins}/**/*.rs').scanSync(root)) {
		if (file.includes('node_modules') || file.includes('/target/')) continue;
		const marks = read(file).match(/allow\(unsafe_code\)/g)?.length ?? 0;
		if (file === ABI ? marks !== MARKS : marks !== 0) problem(file, `${marks} allow(unsafe_code); ${ABI} holds ${MARKS} and no other file any`);
	}
	for (const crate of ours) {
		const home = plugged.get(crate)?.path ?? `crates/${crate}`;
		const toml = read(`${home}/Cargo.toml`);
		if (crate === 'module') {
			if (!/^\[lints\.rust\]\nunsafe_code = "deny"$/m.test(toml)) problem(`${home}/Cargo.toml`, 'the module denies unsafe_code, and allows it on its three names alone');
		} else if (!/^\[lints\]\nworkspace = true$/m.test(toml)) problem(`${home}/Cargo.toml`, 'no `[lints] workspace = true`, so `unsafe` is not forbidden there');
	}

	/* ------------------------------------------- crate bullets match crates/ */

	const layout = read('docs/LAYOUT.md');
	const inCrates = layout.slice(layout.indexOf('## crates/'), layout.indexOf('## The other folders'));
	const crates = cratesIn('crates');
	for (const c of crates) if (!inCrates.includes(`\`${c}\``)) problem('docs/LAYOUT.md § crates/', `crate \`${c}\` has no bullet`);
	for (const m of inCrates.matchAll(/^- `([a-z-]+)`/gm)) {
		if (!crates.includes(m[1])) problem('docs/LAYOUT.md § crates/', `\`${m[1]}\` is not a crate`);
	}
	for (const p of folders) if (!layout.includes(`\`${p}\``)) problem('docs/LAYOUT.md', `plugin \`${p}\` is not named`);

	/* -------------------------------------- bun run names match package.json */

	const scripts = Object.keys(JSON.parse(read('package.json')).scripts as Record<string, string>);
	const listed = [...layout.matchAll(/`bun run ([^`]+)`/g)].map((m) => m[1]).find((l) => l.includes('|'))?.split('|').map((s) => s.trim()) ?? [];
	for (const s of scripts) if (!listed.includes(s)) problem('docs/LAYOUT.md § Commands', `script \`${s}\` is not in the bun run list`);
	for (const s of listed) if (!scripts.includes(s)) problem('docs/LAYOUT.md § Commands', `\`bun run ${s}\` is not a script`);
	for (const file of [...docs, 'AGENTS.md', 'apps/web/AGENTS.md', 'server/AGENTS.md', 'README.md']) {
		for (const m of read(file).matchAll(/bun run ([a-z]+)/g)) {
			if (!scripts.includes(m[1])) problem(file, `\`bun run ${m[1]}\` is not a script`);
		}
	}

	/* ---------------------- a picture's versions are the same size on both sides */

	// The owner's client makes a picture's thumb and low version in the browser
	// and the engine picks a level by their sides: one number each, said in Rust
	// and in TypeScript, held to each other here.
	{
		const rust = read('crates/client/src/pictures.rs');
		const web = read('plugins/items/web/image.ts');
		for (const side of ['THUMB_SIDE', 'LOW_SIDE']) {
			const inRust = rust.match(new RegExp(`^pub const ${side}: u32 = (\\d+);`, 'm'))?.[1];
			const inWeb = web.match(new RegExp(`^export const ${side} = (\\d+);`, 'm'))?.[1];
			if (!inRust || !inWeb) problem(side, 'not found in both crates/client/src/pictures.rs and plugins/items/web/image.ts');
			else if (inRust !== inWeb) problem(side, `${inRust} in crates/client/src/pictures.rs, ${inWeb} in plugins/items/web/image.ts`);
		}
	}

	/* ------------------------------------------------------------ no em dash */

	for (const file of files) {
		read(file)
			.split('\n')
			.forEach((line, i) => {
				if (line.includes('—')) problem(`${file}:${i + 1}`, 'em dash');
			});
	}

	/* ------------------------------------------- an operational doc splits at 200 */

	// The body, under its frontmatter: kb warns a little past this line
	// (kb.config.ts), here it fails. VISION alone keeps its prose, and the
	// guide carries kb's block.
	for (const file of docs) {
		if (file.endsWith('VISION.md')) continue;
		const lines = read(file).replace(/^---\n[\s\S]*?\n---\n+/, '').trimEnd().split('\n').length;
		if (lines > 200) problem(file, `${lines} lines; over 200 it splits by theme`);
	}

	return out;
}
