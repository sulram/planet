// The Go server: build the binary, ensure the dev superuser, serve.
// Bun loads .env from the cwd and spawned processes inherit process.env, so
// the server reads the same APP_URL, SMTP_* and PLANET_OPERATOR_EMAIL.
import { ROOT } from './lib';

const DIR = `${ROOT}/server`;
const BIN = `${DIR}/bin/planet`;

async function run(cmd: string[], cwd: string): Promise<void> {
	const code = await Bun.spawn(cmd, { cwd, stdout: 'inherit', stderr: 'inherit' }).exited;
	if (code !== 0) throw new Error(`${cmd.join(' ')} exited with ${code}`);
}

export async function build(): Promise<void> {
	await run(['go', 'build', '-o', BIN, './cmd/planet'], DIR);
}

/**
 * Upserts the PocketBase superuser (the admin panel login) from the env.
 * Idempotent. Does nothing unless both variables are set.
 */
export async function ensureSuperuser(): Promise<void> {
	const email = process.env.PB_SUPERUSER_EMAIL;
	const password = process.env.PB_SUPERUSER_PASSWORD;
	if (!email || !password) return;
	await run([BIN, 'superuser', 'upsert', email, password], DIR);
}

/** Starts the server on 127.0.0.1:8090 with its data in server/pb_data. */
export function serve() {
	return Bun.spawn([BIN, 'serve'], { cwd: DIR, stdout: 'inherit', stderr: 'inherit' });
}

if (import.meta.main) {
	await build();
	await ensureSuperuser();
	const server = serve();
	process.exitCode = await server.exited;
}
