// Sets a box up for planet, once, as root over ssh. Idempotent: run it again
// after changing the domain or the port, and it rewrites what changed.
//
//   bun run provision production [--dry-run]
//
// What it does: the `planet` system user and /opt/planet, Bun for it (the
// version this Mac runs), Caddy from its own apt repository with a Caddyfile
// for the domain, the firewall rules for 80 and the https port, and the two
// systemd units. What it refuses to do: write the runtime env, which holds
// secrets. /etc/planet/<env>.env is written by hand once (docs/DEPLOY.md).
// The first release, and every release, is `bun run deploy`.
import { envArg, target } from './deploy.config';

const args = process.argv.slice(2);
const dry = args.includes('--dry-run');
const t = target(envArg(args, 'usage: bun run provision <production | staging> [--dry-run]'));

const caddyfile = `# planet, written by scripts/provision.ts. Edit the script, not this file.
${t.httpsPort === 443 ? '' : `{\n\thttps_port ${t.httpsPort}\n}\n\n`}${t.domain}${t.httpsPort === 443 ? '' : `:${t.httpsPort}`} {
	encode zstd gzip
${t.httpsPort === 443 ? '' : '\t# Another service holds 443, so the certificate comes by the HTTP challenge on 80.\n\ttls {\n\t\tissuer acme {\n\t\t\tdisable_tlsalpn_challenge\n\t\t}\n\t}\n'}	# The REST, the ticket and the world socket; then the PocketBase panel.
	handle /api/* {
		reverse_proxy 127.0.0.1:8090
	}
	handle /_/* {
		reverse_proxy 127.0.0.1:8090
	}
	# Everything else is the SvelteKit server.
	handle {
		reverse_proxy 127.0.0.1:3000
	}
}
`;

const serverUnit = `[Unit]
Description=planet world server (PocketBase inside)
After=network-online.target
Wants=network-online.target

[Service]
User=planet
Group=planet
EnvironmentFile=${t.envFile}
WorkingDirectory=${t.dir}/current
ExecStart=${t.dir}/current/planet serve --dir ${t.dir}/pb_data --http 127.0.0.1:8090
Restart=on-failure
RestartSec=2
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
`;

const webUnit = `[Unit]
Description=planet web app (SvelteKit on Bun)
After=network-online.target planet-server.service
Wants=network-online.target

[Service]
User=planet
Group=planet
EnvironmentFile=${t.envFile}
Environment=NODE_ENV=production
WorkingDirectory=${t.dir}/current/web
ExecStart=${t.dir}/.bun/bin/bun ${t.dir}/current/web/index.js
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
`;

// One script, run by bash on the box. Every step is safe to repeat.
const script = `set -euo pipefail
test -s ${t.envFile} || { echo "${t.envFile} is missing: write it first (docs/DEPLOY.md)"; exit 1; }
chmod 600 ${t.envFile}

id -u planet >/dev/null 2>&1 || useradd --system --home-dir ${t.dir} --shell /usr/sbin/nologin planet
mkdir -p ${t.dir}/releases ${t.dir}/pb_data ${t.dir}/assets
chown -R planet:planet ${t.dir}

if [ ! -x ${t.dir}/.bun/bin/bun ] || ! ${t.dir}/.bun/bin/bun --version | grep -qx "${Bun.version}"; then
	echo "installing bun ${Bun.version}"
	curl -fsSL https://bun.sh/install -o /tmp/bun-install.sh
	runuser -u planet -- env HOME=${t.dir} BUN_INSTALL=${t.dir}/.bun bash /tmp/bun-install.sh bun-v${Bun.version} >/dev/null
	rm -f /tmp/bun-install.sh
fi

if ! command -v caddy >/dev/null; then
	echo "installing caddy"
	apt-get install -y -qq debian-keyring debian-archive-keyring apt-transport-https curl >/dev/null
	curl -1sLf https://dl.cloudsmith.io/public/caddy/stable/gpg.key | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg --yes
	curl -1sLf https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt > /etc/apt/sources.list.d/caddy-stable.list
	apt-get update -qq && apt-get install -y -qq caddy >/dev/null
fi
cat > /etc/caddy/Caddyfile <<'CADDY'
${caddyfile}CADDY
caddy validate --config /etc/caddy/Caddyfile >/dev/null
systemctl enable --now caddy >/dev/null
systemctl reload caddy

if command -v ufw >/dev/null; then
	ufw allow 80/tcp comment planet >/dev/null
	ufw allow ${t.httpsPort}/tcp comment planet >/dev/null
fi

cat > /etc/systemd/system/planet-server.service <<'UNIT'
${serverUnit}UNIT
cat > /etc/systemd/system/planet-web.service <<'UNIT'
${webUnit}UNIT
systemctl daemon-reload
systemctl enable planet-server planet-web >/dev/null
echo "provisioned ${t.domain} on ${t.host.split('@').pop()}: next, bun run deploy ${t.env}"
`;

if (dry) {
	console.log(`# would run on ${t.host} as bash -s:\n${script}`);
	process.exit(0);
}
const ssh = Bun.spawn(['ssh', '-o', 'BatchMode=yes', t.host, 'bash -s'], {
	stdin: new Blob([script]),
	stdout: 'inherit',
	stderr: 'inherit'
});
process.exit(await ssh.exited);
