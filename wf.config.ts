// wf (@tekne/wf, from the studio's registry): the flow of parallel agents, each in a herdr
// worktree. Only what is planet's lives here: the worktree's preparation, the gates and
// where to look.
import type { Config } from '@tekne/wf/config';

export default {
	integration: 'dev',
	// a fresh worktree has no node_modules, and the commit hook needs them
	prepare: 'bun install',
	gates: {
		// seconds: the docs against the code, the rules blocks, Rust formatting
		cheap: ['bun run docs', 'bun wf check', 'cargo fmt --all --check'],
		// what CI runs, in CI's order (scripts/check.ts)
		expensive: ['bun run check']
	},
	// no build reads markdown: a diff only in it runs the cheap gates alone
	docsOnly: ['**/*.md'],
	header: "In a worktree, `bun run dev` takes the next free ports: open the URL Vite prints, never localhost:5173, the main checkout's."
} satisfies Config;
