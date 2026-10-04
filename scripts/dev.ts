// Everything at once: the world server and the web front end. Ctrl+C stops both.
// The engine is rebuilt first, every time: cargo is incremental, and a stale
// WASM package silently shows an older world than the desktop does.
import { ROOT } from './lib';
import { build, serve } from './server';
import { buildWasm } from './wasm';

await buildWasm();

await build();
const server = serve();

// Vite holds the page and hands the world's routes and socket to the server,
// so the browser sees one origin, as it does under mundos.
const web = Bun.spawn(['bun', 'run', 'dev'], {
	cwd: `${ROOT}/apps/web`,
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
