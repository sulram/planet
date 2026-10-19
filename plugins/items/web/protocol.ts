/**
 * Items' own part of the seam: the commands `items.take`, `items.paint`,
 * `items.finish`, `items.edge` and `items.state`, and the events `items.hand`
 * and `items.refused`, as the plugin's crate writes them. Pure: no WASM, no DOM.
 */

/** What the menu offers to place. A kind another plugin's session adds enters `kinds.ts` under its own name. */
export type Item = 'cube';
export const items = ['cube'] as const satisfies readonly Item[];

/** How what is placed takes the light: lit, see-through, or shining and lighting what is near. */
export type Finish = 'matte' | 'glass' | 'light';
export const finishes = ['matte', 'glass', 'light'] as const satisfies readonly Finish[];

/** The line drawn around each side of what is placed. */
export type Edge = 'none' | 'black' | 'white';
export const edges = ['none', 'black', 'white'] as const satisfies readonly Edge[];

/** Why an item was not handed over or a part not placed. */
export type Refusal = 'level' | 'pose' | 'gone';
const refusals = ['level', 'pose', 'gone'] as const satisfies readonly Refusal[];

/** `items.hand`: the item in hand, null when none, and the paint what is placed next is made of. */
export interface Hand {
	item: Item | null;
	/** Index into the world's palette. */
	paint: number;
	finish: Finish;
	edge: Edge;
}

const isItem = (value: unknown): value is Item => items.some((item) => item === value);

/** Reads a `hand` event. Null for one that is not a hand. */
export function parseHand(event: Record<string, unknown>): Hand | null {
	const { item, paint } = event;
	if (item !== null && !isItem(item)) return null;
	if (typeof paint !== 'number') return null;
	const finish = finishes.find((finish) => finish === event.finish);
	const edge = edges.find((edge) => edge === event.edge);
	if (!finish || !edge) return null;
	return { item, paint, finish, edge };
}

/** Reads a `refused` event. Null for a reason this front end has no words for. */
export function parseRefused(event: Record<string, unknown>): Refusal | null {
	return refusals.find((reason) => reason === event.reason) ?? null;
}
