/**
 * Typed wrapper over the command/event seam of the engine: the Rust client
 * compiled to WASM by `bun run wasm` into `./pkg` (gitignored, may be absent).
 * The web UI is only a front-end: it sends commands and renders events. Tool
 * logic lives in Rust.
 */
import { parseEvent, type Command, type EngineEvent } from './protocol';

export * from './protocol';

/** A running engine bound to one canvas. `free()` stops its loop and listeners. */
export interface Engine {
	command(command: Command): void;
	/** The ground a recipe names. Sent before the recipe that names it. */
	set_field(bytes: Uint8Array): void;
	/** Opens the link to a world server at a socket URL, the key included. */
	connect(url: string): void;
	disconnect(): void;
	free(): void;
}

/** A loaded engine module, ready to bind to a canvas. */
export interface EngineModule {
	/** Rejects when the browser cannot give the engine a GPU surface. */
	create(canvas: HTMLCanvasElement, onevent: (event: EngineEvent) => void): Promise<Engine>;
}

/** The shape `wasm-bindgen --target web` emits for `shell-web`. */
interface RawEngine {
	command(json: string): void;
	set_field(bytes: Uint8Array): void;
	connect(url: string): void;
	disconnect(): void;
	free(): void;
}
interface RawModule {
	default(): Promise<unknown>;
	Engine: { create(canvas: HTMLCanvasElement, onEvent: (json: string) => void): Promise<RawEngine> };
}

// A glob resolves to nothing when the package was never built, so the app
// type checks, builds and runs without it.
const builds = import.meta.glob<RawModule>('./pkg/shell_web.js');

let loading: Promise<EngineModule | null> | undefined;

async function load(): Promise<EngineModule | null> {
	const build = builds['./pkg/shell_web.js'];
	if (!build) return null;
	const raw = await build();
	await raw.default();
	return {
		async create(canvas, onevent) {
			const engine = await raw.Engine.create(canvas, (json) => {
				const event = parseEvent(json);
				if (event) onevent(event);
			});
			return {
				command: (command) => engine.command(JSON.stringify(command)),
				set_field: (bytes) => engine.set_field(bytes),
				connect: (url) => engine.connect(url),
				disconnect: () => engine.disconnect(),
				free: () => engine.free()
			};
		}
	};
}

/** Loads and initialises the WASM module once. Null when it was never built. */
export function loadEngine(): Promise<EngineModule | null> {
	return (loading ??= load());
}
