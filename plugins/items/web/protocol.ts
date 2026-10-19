/**
 * Items' own part of the seam: the commands `items.edit`, `items.take`,
 * `items.pick`, `items.mode`, `items.set`, `items.remove`, `items.paint`,
 * `items.finish`, `items.edge` and `items.state`, and the events `items.hand`,
 * `items.picked` and `items.refused`, as the plugin's crate writes them.
 * Pure: no WASM, no DOM.
 */

/** What the menu offers to place. A kind another plugin's session adds enters `kinds.ts` under its own name. */
export type Item = 'cube' | 'model' | 'picture';
export const items = ['cube', 'model', 'picture'] as const satisfies readonly Item[];

/** How what is placed takes the light: lit, see-through, or shining and lighting what is near. */
export type Finish = 'matte' | 'glass' | 'light';
export const finishes = ['matte', 'glass', 'light'] as const satisfies readonly Finish[];

/** The line drawn around each side of what is placed. */
export type Edge = 'none' | 'black' | 'white';
export const edges = ['none', 'black', 'white'] as const satisfies readonly Edge[];

/** Why an item was not handed over or a part not placed. */
export type Refusal = 'level' | 'pose' | 'gone' | 'unread';
const refusals = ['level', 'pose', 'gone', 'unread'] as const satisfies readonly Refusal[];

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
}

/** `items.picked`: the part picked as it stands: where, in blocks of its seat; how it is turned, degrees about its own axes; how long its sides are, in blocks; whether it is solid. */
export interface Picked {
	id: number;
	at: [number, number, number];
	rotation: [number, number, number];
	scale: [number, number, number];
	solid: boolean;
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
	return { editing, item, mode, paint, finish, edge, model };
}

const isVector = (value: unknown): value is [number, number, number] =>
	Array.isArray(value) && value.length === 3 && value.every((n) => typeof n === 'number');

/** Reads a `picked` event: the part picked, or null when none is, or when the event is no part. */
export function parsePicked(event: Record<string, unknown>): Picked | null {
	const { id, at, rotation, scale, solid } = event;
	if (typeof id !== 'number') return null;
	if (!isVector(at) || !isVector(rotation) || !isVector(scale) || typeof solid !== 'boolean') return null;
	return { id, at, rotation, scale, solid };
}

/** Reads a `refused` event. Null for a reason this front end has no words for. */
export function parseRefused(event: Record<string, unknown>): Refusal | null {
	return refusals.find((reason) => reason === event.reason) ?? null;
}
