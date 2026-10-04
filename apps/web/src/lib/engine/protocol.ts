/**
 * The command/event seam of the engine, as JSON tagged by `type` (snake_case).
 * Pure: no WASM, no DOM.
 */
import type { Recipe } from '$lib/world';

export type Mode = 'walk' | 'fly';
export const modes = ['walk', 'fly'] as const satisfies readonly Mode[];

/** The most a name carries. The server cuts longer ones. */
export const NAME_CHARS = 24;

/** What a stroke in a volume does. One drag is one stroke. */
export type Tool = 'create' | 'delete' | 'paint';
export const tools = ['create', 'delete', 'paint'] as const satisfies readonly Tool[];

/** The sides a platform can have, in cells. The engine takes the nearest. */
export const platforms = [8, 16, 32, 64] as const;

/** What carries a platform down to the ground: a deck's pillars under an open slab, every column filled, or nothing, a slab that floats. */
export type Base = 'deck' | 'solid' | 'floating';
export const bases = ['deck', 'solid', 'floating'] as const satisfies readonly Base[];

/** Why the engine refused to build: where the body stands, or who it is. */
export type BuildRefusal = 'moon' | 'sea' | 'seam' | 'high' | 'level';

/** What a session may do, as mundos says it and the world repeats it. A builder and an admin build. */
export type Level = 'anonymous' | 'signed_in' | 'builder' | 'admin';

export type ToneMap = 'aces' | 'agx' | 'neutral' | 'reinhard' | 'linear';
export const toneMaps = ['aces', 'agx', 'neutral', 'reinhard', 'linear'] as const satisfies readonly ToneMap[];

/** How the picture is made. Mirrors `scene::Effects`; the engine clamps it. */
export interface Effects {
	shadows: boolean;
	grass: boolean;
	clouds: boolean;
	cloud_cover: number;
	cloud_density: number;
	wind_m_s: number;
	cloud_change: number;
	exposure: number;
	bloom: number;
	bloom_threshold: number;
	haze: number;
	water_clarity: number;
	tone_map: ToneMap;
}

export type Command =
	| { type: 'set_recipe'; recipe: Recipe }
	| { type: 'set_mode'; mode: Mode }
	/** `path` is an asset path from `assets/manifest.json`, such as `avatars/Kyle.vrm`. */
	| { type: 'set_avatar'; path: string }
	| { type: 'random_avatar' }
	| { type: 'next_avatar' }
	/** Fields left out take the engine's default. */
	| { type: 'set_effects'; effects: Partial<Effects> }
	/**
	 * Stand at a place code, `4-K7M42Q`: how a shared address opens where it
	 * says, and how a place someone shared is reached. A short code names a
	 * box and you land in the middle of it.
	 */
	| { type: 'go_to'; place: string }
	/** What to be called: in Hello and, while online, at once. Empty is a name too. */
	| { type: 'set_name'; name: string }
	/** Build with a tool, or stop with null. It builds nothing: a platform is what a stroke starts on. */
	| { type: 'set_tool'; tool: Tool | null }
	/** The side of the platform laid next, in cells. */
	| { type: 'set_platform'; side: number }
	/** Lay a platform where the body stands: a slab as high as the higher of the ground under it and the feet, on a base, a deck when left out. */
	| { type: 'lay_platform'; base?: Base }
	/** The paint the next stroke lays: an index into the palette. */
	| { type: 'set_paint'; paint: number }
	/** Take back the last stroke that landed, or put back the last one taken back. */
	| { type: 'undo' }
	| { type: 'redo' };

/**
 * A plugin's command: its `type` is the plugin's name and the command's,
 * `chat.say`. The engine hands it to that plugin, and refuses it when the
 * plugin is off in this world.
 */
export type PluginCommand = { type: `${string}.${string}`; [field: string]: unknown };

/** A plugin's event, as the plugin wrote it: `chat.said`. */
export type PluginEvent = { type: `${string}.${string}`; [field: string]: unknown };

/** A plugin that is on in this world, as the world's statement says it. */
export interface PluginOn {
	name: string;
	version: number;
}

export type SessionStatus = 'offline' | 'connecting' | 'online';

/** Someone else in the world. The name is their own, or empty. */
export interface PeerInfo {
	session: number;
	name: string;
	visitor: boolean;
}

/** A head on the screen: fractions of the viewport from the top left, and how far. */
export interface Anchor {
	session: number;
	x: number;
	y: number;
	distance_m: number;
}

export type EngineEvent =
	| { type: 'ready'; generator_version: number }
	/** The link to the world server, whenever it changes. */
	/** `level` is null until a world has spoken; its last word stands through a dropped link. */
	| { type: 'session'; status: SessionStatus; session: number | null; level: Level | null }
	/** Who else is here, whenever that changes. Empty when offline. */
	| { type: 'peers'; peers: PeerInfo[] }
	/** The plugins that are on in this world, whenever that changes. Empty when offline. */
	| { type: 'statement'; plugins: PluginOn[] }
	/** Every head in view, yours included, every frame while there is one and once empty after. */
	| { type: 'anchors'; anchors: Anchor[] }
	| { type: 'recipe_changed'; recipe: Recipe }
	/** A command was refused. The message is for logs, not for people. */
	| { type: 'rejected'; message: string }
	/** Nothing is left to build for this view: the world is drawn whole. Again after a leap or a new recipe. */
	| { type: 'settled' }
	| { type: 'mode_changed'; mode: Mode }
	/** The tool in hand, null when not building, the paint it lays, and the side of the next platform. Once at the start too. */
	| { type: 'tool_changed'; tool: Tool | null; paint: number; platform: number }
	/** The paints a cell can take, as `#rrggbb`, in order. Once, at the start. */
	| { type: 'palette'; colors: string[] }
	/** A platform was asked for where no volume can be opened. */
	| { type: 'build_refused'; reason: BuildRefusal }
	/** Whether there is a stroke to take back and one to put back, whenever that changes. */
	| { type: 'history'; undo: boolean; redo: boolean }
	| { type: 'avatar_changed'; path: string }
	| { type: 'effects_changed'; effects: Effects }
	| {
			type: 'stats';
			fps: number;
			altitude_m: number;
			speed_mps: number;
			/** Where the body is, as a person says it: `4-K7M42Q`, `m4-K7M42Q@40` on the moon. */
			place: string;
			/**
			 * The same, plus the way of looking: what a link carries, so whoever
			 * opens it stands where you stood seeing what you saw. Put it in the
			 * address bar and hand it back whole; never take it apart here.
			 */
			pose: string;
			/** Degrees clockwise from north, or null at a pole. */
			bearing_deg: number | null;
	  };

const EVENT_TYPES: ReadonlySet<string> = new Set<EngineEvent['type']>([
	'ready',
	'recipe_changed',
	'rejected',
	'settled',
	'mode_changed',
	'avatar_changed',
	'effects_changed',
	'stats',
	'session',
	'peers',
	'statement',
	'anchors',
	'tool_changed',
	'palette',
	'build_refused',
	'history'
]);

/** Whether an event is a plugin's: its `type` has the plugin's name and a dot before the event's. */
export function isPluginEvent(event: EngineEvent | PluginEvent): event is PluginEvent {
	return event.type.includes('.');
}

/** Parses one event, the core's or a plugin's. Unknown types and malformed payloads yield null. */
export function parseEvent(json: string): EngineEvent | PluginEvent | null {
	try {
		const value: unknown = JSON.parse(json);
		if (typeof value === 'object' && value !== null && 'type' in value && typeof value.type === 'string') {
			if (EVENT_TYPES.has(value.type)) return value as EngineEvent;
			if (/^[a-z_]+\.[a-z_]+$/.test(value.type)) return value as PluginEvent;
		}
	} catch {
		// not JSON: ignored like any unknown event
	}
	return null;
}
