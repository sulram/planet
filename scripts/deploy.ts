// Publishes HEAD to a box: build here, never there; rsync the artefacts into a
// dated release; swap a symlink; restart; check; roll back if the check fails.
//
//   bun run deploy production [--dry-run]
//
// A release is /opt/planet/releases/<stamp>: the Go binary, the web build
// and the one package it imports at run time. The asset set is shared by
// every release at /opt/planet/assets and linked into each, so a deploy
// uploads only what changed of it. The box is set up by `bun run provision`.
import { $ } from 'bun';
import { ROOT } from './lib';
import { envArg, target } from './deploy.config';
import { buildWasm } from './wasm';

const args = process.argv.slice(2);
const dry = args.includes('--dry-run');
const t = target(envArg(args, 'usage: bun run deploy <production | staging> [--dry-run]'));
const RELEASES_KEPT = 5;

$.cwd(ROOT);
const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
const release = `${t.dir}/releases/${stamp}`;
const stage = `${ROOT}/target/deploy`;

function run(cmd: string[]): void {
	console.log('$', cmd.join(' '));
	if (dry) return;
	const p = Bun.spawnSync(cmd, { cwd: ROOT, stdout: 'inherit', stderr: 'inherit' });
	if (p.exitCode !== 0) throw new Error(`${cmd.join(' ')} exited with ${p.exitCode}`);
}
const ssh = (remote: string) => run(['ssh', '-o', 'BatchMode=yes', t.host, remote]);
const sshOk = (remote: string): boolean => {
	console.log('$ ssh', t.host, remote.length > 100 ? `${remote.slice(0, 100)}...` : remote);
	if (dry) return true;
	return Bun.spawnSync(['ssh', '-o', 'BatchMode=yes', t.host, remote], { stdout: 'inherit', stderr: 'inherit' }).exitCode === 0;
};
const capture = (remote: string): string =>
	dry ? '' : Bun.spawnSync(['ssh', '-o', 'BatchMode=yes', t.host, remote], { stdout: 'pipe' }).stdout.toString().trim();
// Links are followed: Bun installs packages as links into its store. The
// flags are the ones macOS's openrsync knows too; ownership is set on the box.
const push = (src: string, dest: string, ...flags: string[]) =>
	run(['rsync', '-azL', ...flags, src, `${t.host}:${dest}`]);

const sha = (await $`git rev-parse --short HEAD`.text()).trim();
const dirty = (await $`git status --porcelain`.text()).trim() !== '';
console.log(`deploy ${t.env}: ${sha}${dirty ? ' (dirty tree)' : ''} -> ${t.host} as ${release}${dry ? ' (dry run)' : ''}`);

// 1. Build, on this machine.
console.log('$ go build (linux/' + t.arch + ')');
if (!dry) {
	await $`rm -rf ${stage}`;
	await $`mkdir -p ${stage}/release/web/node_modules`;
	await $`env GOOS=linux GOARCH=${t.arch} CGO_ENABLED=0 go build -trimpath -ldflags=-s\ -w -o ${stage}/release/planet ./cmd/planet`.cwd(`${ROOT}/server`);
	await buildWasm();
	await $`bun run --cwd apps/web build`;
	// The build without the asset set: that is shared on the box.
	await $`rsync -a --exclude client/assets apps/web/build/ ${stage}/release/web/`;
	// What the server side of the build imports at run time: the app's
	// dependencies, as installed here. devDependencies are bundled.
	const deps = Object.keys((await Bun.file(`${ROOT}/apps/web/package.json`).json()).dependencies ?? {});
	for (const dep of deps) await $`rsync -aL apps/web/node_modules/${dep} ${stage}/release/web/node_modules/`;
}

// 2. Upload: the release, then the shared asset set (only what changed).
ssh(`mkdir -p ${release} && chown planet:planet ${release}`);
push(`${stage}/release/`, `${release}/`);
push(`${ROOT}/assets/`, `${t.dir}/assets/`, '--delete', '--exclude', '.DS_Store');
// The web build serves /assets from client/assets, and its server code reads
// the field sidecars from static/assets beside the build: both are the set.
ssh(
	`ln -sfn ${t.dir}/assets ${release}/web/client/assets && mkdir -p ${release}/static && ln -sfn ${t.dir}/assets ${release}/static/assets && chown -R planet:planet ${release} ${t.dir}/assets`
);

// 3. Swap, restart the server, check it, then the web app, check it.
const previous = capture(`readlink ${t.dir}/current || true`);
ssh(`ln -sfn ${release} ${t.dir}/current && systemctl restart planet-server`);
const serverUp = `for i in $(seq 1 60); do curl -fsS -m 2 http://127.0.0.1:8090/api/health >/dev/null 2>&1 && exit 0; sleep 0.5; done; exit 1`;
const webUp = `for i in $(seq 1 60); do curl -fsS -m 2 -o /dev/null http://127.0.0.1:3000/ 2>/dev/null && exit 0; sleep 0.5; done; exit 1`;
const rollback = (why: string) => {
	if (previous) {
		console.error(`${why}: rolling back to ${previous}`);
		ssh(`ln -sfn ${previous} ${t.dir}/current && systemctl restart planet-server planet-web`);
	} else {
		console.error(`${why}: no previous release to roll back to (journalctl -u planet-server -u planet-web)`);
	}
	throw new Error(why);
};
if (!sshOk(serverUp)) rollback('the server did not come up');
// The panel login, from the env on the box. Idempotent, like scripts/server.ts.
ssh(
	`set -a; . ${t.envFile}; set +a; runuser -u planet -- ${t.dir}/current/planet superuser upsert "$PB_SUPERUSER_EMAIL" "$PB_SUPERUSER_PASSWORD" --dir ${t.dir}/pb_data >/dev/null`
);
ssh('systemctl restart planet-web');
if (!sshOk(webUp)) rollback('the web app did not come up');

// 4. Record, and keep the last few releases for a rollback by hand.
ssh(`echo ${sha}${dirty ? '-dirty' : ''} > ${t.dir}/DEPLOYED_SHA && cd ${t.dir}/releases && ls -1dt */ | tail -n +${RELEASES_KEPT + 1} | xargs -r rm -rf`);
console.log(`deployed ${sha} to ${t.origin}`);
