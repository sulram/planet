/**
 * Pure world types and seed helpers. No PocketBase, no DOM.
 * Field names match the wire form shared by the engine and the `worlds`
 * collection, so a Recipe travels unmapped.
 */

/**
 * What gives a world its shape. Both still take the seed: a field carries the
 * low frequencies of a real body, and everything under one of its texels is
 * the generator's, so one field is a family of worlds, never a single world.
 */
export type Source = 'generated' | { field: string };

/** What a person picks in the UI, before it becomes a `Source`. */
export const SHAPES = ['generated', 'earth'] as const;
export type Shape = (typeof SHAPES)[number];

export function isShape(value: string | null): value is Shape {
	return SHAPES.includes(value as Shape);
}

/** A baked field, as its sidecar names it. `bun run field` writes both. */
export interface Field {
	id: string;
	texel_m: number;
	source: string;
}

/** Where a shape's field and its sidecar are served from. */
export const FIELD_PATH: Record<Exclude<Shape, 'generated'>, string> = {
	earth: '/assets/fields/earth.field'
};

/** The id a recipe names, or null when the seed alone shapes the world. */
export function fieldId(recipe: Recipe): string | null {
	const source = recipe.params.source;
	return typeof source === 'object' && source !== null && 'field' in source
		? String((source as { field: unknown }).field)
		: null;
}

/** Seed + params + generator version: enough to regenerate all untouched terrain. */
export interface Recipe {
	/** A u64 as 16 lowercase hex digits. */
	seed: string;
	generator_version: number;
	params: Record<string, unknown>;
}

export interface World {
	id: string;
	name: string;
	recipe: Recipe;
	owner: string;
	created: string;
	updated: string;
}

export const WORLD_NAME_MAX = 80;

const SEED = /^[0-9a-f]{16}$/;

export function isSeed(value: string): boolean {
	return SEED.test(value);
}

/**
 * Accepts what a person may type or paste: surrounding space, upper case, a
 * `0x` prefix, fewer than 16 digits (left padded, as a number would be).
 * Returns the canonical seed, or null when the input is not a u64 in hex.
 */
export function normalizeSeed(raw: string | null | undefined): string | null {
	if (!raw) return null;
	const digits = raw.trim().toLowerCase().replace(/^0x/, '');
	if (!/^[0-9a-f]{1,16}$/.test(digits)) return null;
	return digits.padStart(16, '0');
}

/** 8 bytes from the platform CSPRNG, as a seed. */
export function randomSeed(): string {
	const bytes = crypto.getRandomValues(new Uint8Array(8));
	return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

export function isGeneratorVersion(value: number): boolean {
	return Number.isInteger(value) && value >= 1;
}
