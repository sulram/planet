/**
 * The command/event seam of the engine, as JSON tagged by `type` (snake_case).
 * Pure: no WASM, no DOM.
 */
import type { Recipe } from '$lib/world';

export type Mode = 'walk' | 'fly';
export const modes = ['walk', 'fly'] as const satisfies readonly Mode[];

export type Command = { type: 'set_recipe'; recipe: Recipe } | { type: 'set_mode'; mode: Mode };

export type EngineEvent =
	| { type: 'ready'; generator_version: number }
	| { type: 'recipe_changed'; recipe: Recipe }
	| { type: 'mode_changed'; mode: Mode }
	| { type: 'stats'; fps: number; altitude_m: number; speed_mps: number; sector: number };

const EVENT_TYPES: ReadonlySet<string> = new Set<EngineEvent['type']>([
	'ready',
	'recipe_changed',
	'mode_changed',
	'stats'
]);

/** Parses one event. Unknown types and malformed payloads yield null. */
export function parseEvent(json: string): EngineEvent | null {
	try {
		const value: unknown = JSON.parse(json);
		if (typeof value === 'object' && value !== null && 'type' in value && typeof value.type === 'string') {
			if (EVENT_TYPES.has(value.type)) return value as EngineEvent;
		}
	} catch {
		// not JSON: ignored like any unknown event
	}
	return null;
}
