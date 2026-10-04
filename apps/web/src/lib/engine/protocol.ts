/**
 * The command/event seam of the engine, as JSON tagged by `type` (snake_case).
 * Pure: no WASM, no DOM.
 */
import type { Recipe } from '$lib/world';

export type Mode = 'walk' | 'fly';
export const modes = ['walk', 'fly'] as const satisfies readonly Mode[];

/** The most a name carries. The server cuts longer ones. */
export const NAME_CHARS = 24;

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
	| { type: 'set_name'; name: string };

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
	/** The paints a cell can take, as `#rrggbb`, in order: the world's palette. Once, at the start. */
	| { type: 'palette'; colors: string[] }
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
	'palette'
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
