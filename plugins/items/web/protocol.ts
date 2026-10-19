/**
 * Items' own part of the seam: the commands `items.edit`, `items.take`,
 * `items.pick`, `items.mode`, `items.set`, `items.remove`, `items.paint`,
 * `items.finish`, `items.edge`, `items.state`, `items.list` and
 * `items.chart`, and the events `items.hand`, `items.picked`,
 * `items.refused`, `items.near`, `items.landmarks` and `items.crossed`,
 * as the plugin's crate writes them. Pure: no WASM, no DOM.
 */

/** What the menu offers to place. A kind another plugin's session adds enters `kinds.ts` under its own name. */
export type Item = 'cube' | 'model' | 'picture' | 'landmark' | 'portal';
export const items = ['cube', 'model', 'picture', 'landmark', 'portal'] as const satisfies readonly Item[];

/** How what is placed takes the light: lit, see-through, or shining and lighting what is near. */
export type Finish = 'matte' | 'glass' | 'light';
export const finishes = ['matte', 'glass', 'light'] as const satisfies readonly Finish[];

/** The line drawn around each side of what is placed. */
export type Edge = 'none' | 'black' | 'white';
export const edges = ['none', 'black', 'white'] as const satisfies readonly Edge[];

/** Why an item was not handed over or a part not placed. */
export type Refusal = 'level' | 'pose' | 'gone' | 'unread' | 'name' | 'nowhere';
const refusals = ['level', 'pose', 'gone', 'unread', 'name', 'nowhere'] as const satisfies readonly Refusal[];

/** What a drag of a handle does to the part picked. */
export type Mode = 'move' | 'rotate' | 'scale';
export const modes = ['move', 'rotate', 'scale'] as const satisfies readonly Mode[];

/** `items.hand`: whether the menu is open, the item in hand, null when none, the mode, and the paint what is placed next is made of. */
export interface Hand {
	editing: boolean;
	item: Item | null;
	mode: Mode;
	/** Index into the world's palette. */
	paint: number;
	finish: Finish;
	edge: Edge;
	/** The model in hand, by its file's asset name. Null with none taken. */
	model: string | null;
	/** Whether the model in hand is still on its way: it places a glass box until its file arrives. */
	loading: boolean;
	/** The name the next landmark bears, once one was given. */
	name: string | null;
	/** The landmark the next portal leads to, by its name in the world, once one was chosen. */
	to: number | null;
}

/** `items.picked`: the part picked as it stands: where, in blocks of its seat; how it is turned, degrees about its own axes; how long its sides are, in blocks; whether it is solid. */
export interface Picked {
	id: number;
	/** What kind of item it is. */
	item: Item;
	at: [number, number, number];
	rotation: [number, number, number];
	scale: [number, number, number];
	solid: boolean;
	/** How much it shines with its own colour: 0 lit, 1 a screen, more a lamp. */
	shine: number;
	/** How far its light goes, metres; 0 is by its size. */
	reach_m: number;
	/** Whether it is a lamp: a cube in the light finish, or a landmark. */
	lamp: boolean;
	/** What a landmark is called. */
	name?: string;
	/** Where a portal leads: a landmark's name in the world, null when it leads nowhere. */
	to?: number | null;
}

const isItem = (value: unknown): value is Item => items.some((item) => item === value);

/** Reads a `hand` event. Null for one that is not a hand. */
export function parseHand(event: Record<string, unknown>): Hand | null {
	const { editing, item, paint } = event;
	if (typeof editing !== 'boolean') return null;
	if (item !== null && !isItem(item)) return null;
	if (typeof paint !== 'number') return null;
	const mode = modes.find((mode) => mode === event.mode);
	const finish = finishes.find((finish) => finish === event.finish);
	const edge = edges.find((edge) => edge === event.edge);
	if (!mode || !finish || !edge) return null;
	const model = typeof event.model === 'string' ? event.model : null;
	const loading = event.loading === true;
	const name = typeof event.name === 'string' ? event.name : null;
	const to = typeof event.to === 'number' ? event.to : null;
	return { editing, item, mode, paint, finish, edge, model, loading, name, to };
}

const isVector = (value: unknown): value is [number, number, number] =>
	Array.isArray(value) && value.length === 3 && value.every((n) => typeof n === 'number');

/** Reads a `picked` event: the part picked, or null when none is, or when the event is no part. */
export function parsePicked(event: Record<string, unknown>): Picked | null {
	const { id, item, at, rotation, scale, solid, shine, reach_m, lamp, name, to } = event;
	if (typeof id !== 'number' || !isItem(item)) return null;
	if (!isVector(at) || !isVector(rotation) || !isVector(scale) || typeof solid !== 'boolean' || typeof shine !== 'number' || typeof reach_m !== 'number' || typeof lamp !== 'boolean') return null;
	const picked: Picked = { id, item, at, rotation, scale, solid, shine, reach_m, lamp };
	if (typeof name === 'string') picked.name = name;
	if ('to' in event) picked.to = typeof to === 'number' ? to : null;
	return picked;
}

/** `items.landmarks`: one landmark of the world: its name in the world, what it is called, its paint, how far in metres, null on another body, and the place code that takes a body to it. */
export interface LandmarkOne {
	id: number;
	name: string;
	paint: number;
	away_m: number | null;
	place: string;
}

/** Reads a `landmarks` event: the landmarks of the world, nearest first. Null for one that is no chart. */
export function parseLandmarks(event: Record<string, unknown>): LandmarkOne[] | null {
	const { landmarks } = event;
	if (!Array.isArray(landmarks)) return null;
	const rows: LandmarkOne[] = [];
	for (const row of landmarks) {
		if (typeof row !== 'object' || row === null) return null;
		const { id, name, paint, away_m, place } = row as Record<string, unknown>;
		if (typeof id !== 'number' || typeof name !== 'string' || typeof paint !== 'number' || typeof place !== 'string') return null;
		rows.push({ id, name, paint, away_m: typeof away_m === 'number' ? away_m : null, place });
	}
	return rows;
}

/** `items.near`: one part near the body: its name, what kind of item it is, how far in metres, its paint and whether it is solid. */
export interface NearOne {
	id: number;
	item: Item;
	away_m: number;
	paint: number;
	solid: boolean;
}

/** Reads a `near` event: the parts near the body, nearest first. Null for one that is no list. */
export function parseNear(event: Record<string, unknown>): NearOne[] | null {
	const { near } = event;
	if (!Array.isArray(near)) return null;
	const rows: NearOne[] = [];
	for (const row of near) {
		if (typeof row !== 'object' || row === null) return null;
		const { id, item, away_m, paint, solid } = row as Record<string, unknown>;
		if (typeof id !== 'number' || !isItem(item) || typeof away_m !== 'number' || typeof paint !== 'number' || typeof solid !== 'boolean') return null;
		rows.push({ id, item, away_m, paint, solid });
	}
	return rows;
}

/** Reads a `refused` event. Null for a reason this front end has no words for. */
export function parseRefused(event: Record<string, unknown>): Refusal | null {
	return refusals.find((reason) => reason === event.reason) ?? null;
}
