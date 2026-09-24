// Where an environment is deployed: read from the environment, never from
// git, so moving to another box is a change of `.env` and nothing here.
//
// Per environment, prefixed `DEPLOY_<ENV>_`:
//   HOST        root@box, reachable by ssh key
//   DOMAIN      the public name, with an A record at the box
//   HTTPS_PORT  443, or another port while something else holds 443 (default 443)
//   ARCH        amd64 or arm64, the box's (default amd64)
//
// On the box, one env file per environment holds the runtime settings and
// the secrets: /etc/planet/<env>.env, written by hand once (docs/DEPLOY.md).
export type Env = 'production' | 'staging';
export const environments: Env[] = ['production', 'staging'];

export type Target = {
	env: Env;
	host: string;
	domain: string;
	httpsPort: number;
	arch: 'amd64' | 'arm64';
	/** The public origin, as APP_URL and PB_PUBLIC_URL say it on the box. */
	origin: string;
	/** Where everything lives on the box. */
	dir: string;
	/** The runtime env on the box. */
	envFile: string;
};

export function target(env: Env): Target {
	const prefix = `DEPLOY_${env.toUpperCase()}`;
	const need = (key: string): string => {
		const value = process.env[`${prefix}_${key}`];
		if (!value) throw new Error(`${prefix}_${key} is not set: put it in .env (see .env.example)`);
		return value;
	};
	const httpsPort = Number(process.env[`${prefix}_HTTPS_PORT`] || 443);
	if (!Number.isInteger(httpsPort) || httpsPort <= 0) throw new Error(`${prefix}_HTTPS_PORT is not a port`);
	const arch = process.env[`${prefix}_ARCH`] || 'amd64';
	if (arch !== 'amd64' && arch !== 'arm64') throw new Error(`${prefix}_ARCH is amd64 or arm64`);
	const domain = need('DOMAIN');
	return {
		env,
		host: need('HOST'),
		domain,
		httpsPort,
		arch,
		origin: `https://${domain}${httpsPort === 443 ? '' : `:${httpsPort}`}`,
		dir: '/opt/planet',
		envFile: `/etc/planet/${env}.env`
	};
}

/** The environment named on the command line, or a usage error. */
export function envArg(args: string[], usage: string): Env {
	const env = args.find((a) => !a.startsWith('--')) as Env | undefined;
	if (!env || !environments.includes(env)) {
		console.error(usage);
		process.exit(1);
	}
	return env;
}
