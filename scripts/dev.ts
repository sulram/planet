// Everything at once: the world server and the web front end. Ctrl+C stops both.
// The engine is rebuilt first, every time: cargo is incremental, and a stale
// WASM package silently shows an older world than the desktop does.
import { HOST, ROOT, freePort } from './lib';
import { build, serve } from './server';
import { buildWasm } from './wasm';

await buildWasm();

await build();
// Several checkouts run at once, each with its own world: the server takes
// the usual port when it is free and the next free one when it is not, and
// Vite does the same for the page on its own.
const port = freePort(Number(process.env.PORT ?? 8090));
const server = serve(port);

// Vite holds the page and hands the world's routes and socket to the server,
// so the browser sees one origin, as it does under mundos.
const web = Bun.spawn(['bun', 'run', 'dev'], {
	cwd: `${ROOT}/apps/web`,
	env: { ...process.env, WORLD_ORIGIN: `http://${HOST}:${port}` },
	stdout: 'inherit',
	stderr: 'inherit'
});

const stop = () => {
	server.kill();
	web.kill();
	process.exit(0);
};
process.on('SIGINT', stop);
process.on('SIGTERM', stop);

// Either one going takes the other down.
await Promise.race([server.exited, web.exited]);
stop();
