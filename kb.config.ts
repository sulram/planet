// kb (@tekne/kb, the studio's registry): the docs, their decisions and open
// questions, and the shared agent rules in AGENTS.md. The checks only this
// repo needs are in scripts/kb-plugins.ts.
import { defineConfig } from '@tekne/kb/config';

export default defineConfig({
	// Sketches are drawn ahead and nothing in them is decided: not topic docs.
	// A doc's body stops at 200 lines (scripts/kb-plugins.ts); its frontmatter is the rest.
	docs: { groups: ['Why', 'Shape', 'Work', 'Reference'], skip: ['sketches'], maxLines: 210 },
	questions: {
		groups: {
			marlus: 'Waits on Marlus',
			hosting: 'Blocks hosting',
			plugins: 'Blocks plugins',
			building: 'Blocks building: the address is the save format, settled before the first volume is stored',
			later: 'Later',
			problems: 'Problems with no chosen fix'
		},
		staleDays: 90
	},
	// Comments that point at docs (`docs/x.md § Section`) point somewhere real.
	code: { glob: '{crates/**/*.rs,plugins/**/*.{rs,ts,svelte},server/**/*.go,apps/web/src/**/*.{ts,svelte},scripts/*.ts}' },
	agents: { modules: ['core', 'worktrees'] },
	plugins: () => import('./scripts/kb-plugins')
});
