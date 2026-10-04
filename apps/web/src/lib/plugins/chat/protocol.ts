/**
 * Chat's own part of the seam: the command `chat.say` and the event
 * `chat.said`, as `crates/chat` writes them. Pure: no WASM, no DOM.
 */

/** Who hears a line: within reach on the same body, or the whole world. Never another world. */
export type Scope = 'near' | 'world';
export const scopes = ['near', 'world'] as const satisfies readonly Scope[];
/** The most a line carries, in characters (code points). The server drops longer ones unheard. */
export const LINE_CHARS = 500;

/**
 * `chat.said`: a line someone said, your own included, so what the world
 * heard is what is shown. `place` is where the speaker stood when they shared
 * it, ready for `go_to`, or null.
 */
export interface Said {
	session: number;
	scope: Scope;
	text: string;
	place: string | null;
}

/** Reads a `said` event. Null for one that is not a line. */
export function parseSaid(event: Record<string, unknown>): Said | null {
	const { session, scope, text, place } = event;
	if (typeof session !== 'number' || typeof text !== 'string') return null;
	if (scope !== 'near' && scope !== 'world') return null;
	if (place !== null && typeof place !== 'string') return null;
	return { session, scope, text, place };
}

/** One line as the bar and the balloons show it: who said it, with the name they had then. */
export interface Line extends Said {
	id: number;
	who: string;
	own: boolean;
	/** When it was heard, `Date.now()`. */
	at: number;
}
