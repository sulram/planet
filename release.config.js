// Semantic Release. The why lives in docs/DECISIONS.md 21.
//
// - `dev` publishes prereleases: 0.0.1-dev.1, 0.0.1-dev.2, ...
// - merging `dev` into `main` publishes the plain version: 0.0.1
// - the tag `v0.0.0` on the first commit anchors the count below 1.0.0
//
// Pre-1.0 rules: nothing reaches 1.0.0 by accident.
//   BREAKING CHANGE -> minor, feat / fix / perf -> patch.
// At 1.0.0 delete `releaseRules` and the defaults apply
// (BREAKING CHANGE major, feat minor, fix patch).
export default {
	branches: ['main', { name: 'dev', prerelease: 'dev' }],
	tagFormat: 'v${version}',
	plugins: [
		[
			'@semantic-release/commit-analyzer',
			{
				preset: 'conventionalcommits',
				releaseRules: [
					{ breaking: true, release: 'minor' },
					{ type: 'feat', release: 'patch' },
					{ type: 'fix', release: 'patch' },
					{ type: 'perf', release: 'patch' }
				]
			}
		],
		['@semantic-release/release-notes-generator', { preset: 'conventionalcommits' }],
		'@semantic-release/github'
	]
};
