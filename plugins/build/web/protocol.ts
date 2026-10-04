/**
 * Building's own part of the seam: the commands `build.take`, `build.paint`,
 * `build.platform`, `build.lay`, `build.close`, `build.undo`, `build.redo`
 * and `build.state`, and the events `build.hand`, `build.over`,
 * `build.refused` and `build.history`, as the plugin's crate writes them.
 * Pure: no WASM, no DOM.
 */

/** What a stroke in a volume does. One drag is one stroke. */
export type Tool = 'create' | 'delete' | 'paint';
export const tools = ['create', 'delete', 'paint'] as const satisfies readonly Tool[];

/** The sides a platform can have, in cells. The engine takes the nearest. */
export const platforms = [8, 16, 32, 64] as const;

/** What carries a platform down to the ground: a deck's pillars under an open slab, every column filled, or nothing, a slab that floats. */
export type Base = 'deck' | 'solid' | 'floating';
export const bases = ['deck', 'solid', 'floating'] as const satisfies readonly Base[];

/** Why a tool was not handed over, no platform laid or no volume deleted: where the body stands, or who it is. */
export type Refusal = 'moon' | 'sea' | 'seam' | 'high' | 'level' | 'field' | 'empty';
const refusals = ['moon', 'sea', 'seam', 'high', 'level', 'field', 'empty'] as const satisfies readonly Refusal[];

/** `build.hand`: the tool in hand, null when not building, the paint it lays, and the side of the next platform. */
export interface Hand {
	tool: Tool | null;
	/** Index into the world's palette. */
	paint: number;
	platform: number;
}

/** `build.history`: whether there is a change to take back, and one to put back. */
export interface History {
	undo: boolean;
	redo: boolean;
}

const isTool = (value: unknown): value is Tool => tools.some((tool) => tool === value);

/** Reads a `hand` event. Null for one that is not a hand. */
export function parseHand(event: Record<string, unknown>): Hand | null {
	const { tool, paint, platform } = event;
	if (tool !== null && !isTool(tool)) return null;
	if (typeof paint !== 'number' || typeof platform !== 'number') return null;
	return { tool, paint, platform };
}

/** Reads an `over` event: whether a volume stands under the body. Null for one that does not say. */
export function parseOver(event: Record<string, unknown>): boolean | null {
	return typeof event.volume === 'boolean' ? event.volume : null;
}

/** Reads a `refused` event. Null for a reason this front end has no words for. */
export function parseRefused(event: Record<string, unknown>): Refusal | null {
	return refusals.find((reason) => reason === event.reason) ?? null;
}

/** Reads a `history` event. Null for one that is not a history. */
export function parseHistory(event: Record<string, unknown>): History | null {
	const { undo, redo } = event;
	if (typeof undo !== 'boolean' || typeof redo !== 'boolean') return null;
	return { undo, redo };
}
