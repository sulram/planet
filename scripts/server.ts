// The Go server: build the module it embeds, then the binary, and serve one
// world from server/world.
// Bun loads .env from the cwd and spawned processes inherit process.env, so
// the server reads the same variables (.env.example).
import { ROOT } from './lib';
import { buildModule } from './module';

const DIR = `${ROOT}/server`;
const BIN = `${DIR}/bin/planet`;

async function run(cmd: string[], cwd: string): Promise<void> {
	const code = await Bun.spawn(cmd, { cwd, stdout: 'inherit', stderr: 'inherit' }).exited;
	if (code !== 0) throw new Error(`${cmd.join(' ')} exited with ${code}`);
}

export async function build(): Promise<void> {
	// The world halves are part of the binary: cargo is incremental, and a
	// stale module silently runs an older plugin than the engine holds.
	await buildModule();
	await run(['go', 'build', '-o', BIN, './cmd/planet'], DIR);
}

/**
 * Starts the server on 127.0.0.1:8090 with its world in server/world. Alone,
 * every session is an admin, or the level `.env` names: the one at the
 * keyboard founds the world and builds. With mundos's three variables in
 * `.env`, mundos says the level.
 */
export function serve() {
	const env: Record<string, string | undefined> = { WORLD_DIR: `${DIR}/world`, ...process.env };
	if (!env.MUNDOS_PUBLIC_KEY) env.PLANET_DEV_LEVEL ??= 'admin';
	return Bun.spawn([BIN, 'serve'], { cwd: DIR, env, stdout: 'inherit', stderr: 'inherit' });
}

if (import.meta.main) {
	await build();
	const server = serve();
	process.exitCode = await server.exited;
}
