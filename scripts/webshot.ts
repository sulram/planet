// What the browser build shows and says: opens a page of the running dev
// server in headless Chrome (WebGPU on), prints its console and saves a PNG.
// The desktop shot cannot see a browser only failure; this can.
//   bun run dev            (in another terminal)
//   bun run webshot        -> out/web.png, from the main world
//   bun run webshot --path "/#4-K7M42Q" --wait 20 --out out/web.png
//   bun run webshot --eval "document.querySelector('.toggle button').click()"
//   bun run webshot --port 9334   (a second Chrome beside the first: two people in one world)
//   bun run webshot --eval "..." --after 6   (seconds between the script and the picture)
//   bun run webshot --eval "..." --film 2   (every frame painted for two seconds after the script: out/web-000.png on, their times in out/web.txt)
//   bun run webshot --lang pt-BR   (the page in the language a browser of that locale asks for)
//   bun run webshot --profile out/ada   (a browser kept between runs: what a page kept is there on the next)
import { mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { ROOT } from './lib';

const args = process.argv.slice(2);
const option = (name: string, fallback: string) => {
	const at = args.indexOf(`--${name}`);
	return at >= 0 && args[at + 1] ? args[at + 1] : fallback;
};
const url = `${option('origin', 'http://[::1]:5173')}${option('path', '/')}`;
const out = resolve(ROOT, option('out', 'out/web.png'));
const waitMs = Number(option('wait', '15')) * 1000;
// Run in the page once it has settled, before the picture: a click, a walk.
const script = option('eval', '');
const afterMs = Number(option('after', '1')) * 1000;
// A motion is a film: every frame the page paints after the script, each
// named by its order and said with its time since the script.
const filmMs = Number(option('film', '0')) * 1000;
const PORT = Number(option('port', '9333'));
const lang = option('lang', '');
// A fresh browser each run, unless one is named to keep.
const profile = args.includes('--profile') ? resolve(ROOT, option('profile', '')) : mkdtempSync(join(tmpdir(), 'planet-webshot-'));
mkdirSync(profile, { recursive: true });

const binary =
	process.env.CHROME ??
	Bun.which('google-chrome') ??
	Bun.which('chromium') ??
	'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const chrome = Bun.spawn(
	[
		binary,
		'--headless=new',
		`--remote-debugging-port=${PORT}`,
		'--enable-unsafe-webgpu',
		`--user-data-dir=${profile}`,
		'--window-size=1280,720',
		...(lang ? [`--lang=${lang}`, `--accept-lang=${lang}`] : []),
		'about:blank'
	],
	{ stdout: 'ignore', stderr: 'ignore' }
);

let filming = 0;
const times: string[] = [];
function film(params: { data: string; sessionId: number }) {
	socket.send(JSON.stringify({ id: ++sent, method: 'Page.screencastFrameAck', params: { sessionId: params.sessionId } }));
	if (!filming) return;
	const frame = out.replace(/\.png$/, `-${String(times.length).padStart(3, '0')}.png`);
	mkdirSync(dirname(frame), { recursive: true });
	Bun.write(frame, Buffer.from(params.data, 'base64'));
	times.push(`${frame}\t${((performance.now() - filming) / 1000).toFixed(2)} s`);
}

type Target = { type: string; webSocketDebuggerUrl: string };
let targets: Target[] = [];
for (let attempt = 0; attempt < 40 && targets.length === 0; attempt++) {
	await Bun.sleep(250);
	targets = await fetch(`http://localhost:${PORT}/json`)
		.then((response) => response.json() as Promise<Target[]>)
		.catch(() => []);
}
const page = targets.find((target) => target.type === 'page');
if (!page) {
	chrome.kill();
	throw new Error(`Chrome did not start (${binary}). Set CHROME to its path.`);
}

const socket = new WebSocket(page.webSocketDebuggerUrl);
let sent = 0;
const waiting = new Map<number, (result: any) => void>();
const send = (method: string, params = {}) =>
	new Promise<any>((done) => {
		waiting.set(++sent, done);
		socket.send(JSON.stringify({ id: sent, method, params }));
	});
socket.onmessage = (event) => {
	const message = JSON.parse(event.data as string);
	if (message.id) return waiting.get(message.id)?.(message.result);
	const { method, params } = message;
	if (method === 'Page.screencastFrame') return film(params);
	if (method === 'Runtime.consoleAPICalled') {
		const text = params.args.map((arg: any) => arg.value ?? arg.description ?? '').join(' ');
		console.log(`[${params.type}] ${text}`);
	} else if (method === 'Runtime.exceptionThrown') {
		console.log(`[exception] ${JSON.stringify(params.exceptionDetails)}`);
	} else if (method === 'Log.entryAdded') {
		console.log(`[${params.entry.level}] ${params.entry.text}`);
	}
};
await new Promise((open) => (socket.onopen = open));
await send('Runtime.enable');
await send('Log.enable');
await send('Page.enable');
await send('Page.navigate', { url });
await Bun.sleep(waitMs);
if (filmMs > 0) {
	await send('Page.startScreencast', { format: 'png' });
	filming = performance.now();
	if (script) await send('Runtime.evaluate', { expression: script });
	await Bun.sleep(filmMs);
	await send('Page.stopScreencast');
	filming = 0;
	const index = out.replace(/\.png$/, '.txt');
	await Bun.write(index, `${times.join('\n')}\n`);
	console.log(`${times.length} frames -> ${index}`);
} else if (script) {
	await send('Runtime.evaluate', { expression: script });
	await Bun.sleep(afterMs);
}
const shot = await send('Page.captureScreenshot', { format: 'png' });
mkdirSync(dirname(out), { recursive: true });
await Bun.write(out, Buffer.from(shot.data, 'base64'));
console.log(`${url} -> ${out}`);
socket.close();
// A kept browser is closed as a person closes it, so what the page kept is
// written down before it goes; a signal loses it.
if (args.includes('--profile')) {
	const { webSocketDebuggerUrl } = (await fetch(`http://localhost:${PORT}/json/version`).then((response) => response.json())) as { webSocketDebuggerUrl: string };
	const browser = new WebSocket(webSocketDebuggerUrl);
	await new Promise((open) => (browser.onopen = open));
	browser.send(JSON.stringify({ id: 1, method: 'Browser.close' }));
	await Promise.race([chrome.exited, Bun.sleep(5000)]);
}
chrome.kill();
process.exit(0);
