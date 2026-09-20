// Conventional Commits, as written in CLAUDE.md "Commits".
// Two house rules ride along as a local plugin: no em dashes, no AI trailers.
const AI_TRAILER = /co-authored-by:.*(claude|codex|kimi|copilot|anthropic|openai)|generated with/i;

export default {
	extends: ['@commitlint/config-conventional'],
	plugins: [
		{
			rules: {
				'no-em-dash': ({ raw }) => [!raw.includes('\u2014'), 'no em dashes (CLAUDE.md, Docs style)'],
				'no-ai-trailer': ({ raw }) => [
					!AI_TRAILER.test(raw),
					'no AI or tool co-author trailer, no "generated with" footer (CLAUDE.md, Commits)'
				]
			}
		}
	],
	rules: {
		'type-enum': [2, 'always', ['feat', 'fix', 'docs', 'chore', 'refactor', 'test', 'perf']],
		'no-em-dash': [2, 'always'],
		'no-ai-trailer': [2, 'always']
	}
};
