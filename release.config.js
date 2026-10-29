// Semantic Release. The why is DECISIONS releases-from-1 (docs/decisions/).
//
// - `dev` publishes prereleases: 1.0.0-dev.1, 1.0.0-dev.2, ...
// - merging `dev` into `main` publishes the plain version: 1.0.0 first
// - the commit types bump as the analyzer's defaults say:
//   BREAKING CHANGE major, feat minor, fix and perf patch
export default {
	branches: ['main', { name: 'dev', prerelease: 'dev' }],
	tagFormat: 'v${version}',
	plugins: [
		['@semantic-release/commit-analyzer', { preset: 'conventionalcommits' }],
		['@semantic-release/release-notes-generator', { preset: 'conventionalcommits' }],
		'@semantic-release/github'
	]
};
