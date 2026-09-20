// Everything at once: server (PocketBase inside) + Mailpit + web. Ctrl+C stops all.
// The engine is built first when missing; rebuild it with `bun run wasm`.
import { existsSync } from 'node:fs';
import { ROOT } from './lib';
import { build, ensureSuperuser, serve } from './server';
import { buildWasm } from './wasm';

if (!existsSync(`${ROOT}/apps/web/src/lib/engine/pkg/shell_web.js`)) await buildWasm();

await build();
await ensureSuperuser();
const server = serve();

// Mailpit catches dev email (http://localhost:8025) when SMTP_HOST points at it.
// Without it the server prints each magic link to this console instead.
const mailpitBin = process.env.SMTP_HOST ? Bun.which('mailpit') : null;
const mailpit = mailpitBin ? Bun.spawn([mailpitBin], { stdout: 'ignore', stderr: 'inherit' }) : null;
if (mailpit) console.log('mail -> http://localhost:8025');

const web = Bun.spawn(['bun', 'run', 'dev'], {
	cwd: `${ROOT}/apps/web`,
	stdout: 'inherit',
	stderr: 'inherit'
});

const stop = () => {
	server.kill();
	mailpit?.kill();
	web.kill();
	process.exit(0);
};
process.on('SIGINT', stop);
process.on('SIGTERM', stop);

// The server and the web app take the set down; Mailpit dying does not.
await Promise.race([server.exited, web.exited]);
stop();
