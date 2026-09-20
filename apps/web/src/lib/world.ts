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

/**
 * The knobs a person turns when a planet is made, in the order they read.
 *
 * One table, because every other place that touches them (the URL, the
 * sliders, the form, the clamp on the server) has to agree, and agreeing is
 * cheaper than checking. `shapes` says which sources a knob means anything
 * for: a field fixes its own coastline, so the two that place one are for
 * generated worlds, and the two that shape a sea floor out of a real one are
 * for fields.
 */
export type KnobKey =
	| 'relief_m'
	| 'ocean_depth_m'
	| 'continent_scale'
	| 'sea_share'
	| 'sea_level_m'
	| 'sea_curve';

export interface Knob {
	key: KnobKey;
	min: number;
	max: number;
	step: number;
	fallback: number;
	/** Decimals in the URL and on the slider. */
	places: number;
	shapes: readonly Shape[];
}

export const KNOBS: readonly Knob[] = [
	{ key: 'relief_m', min: 200, max: 4000, step: 50, fallback: 1400, places: 0, shapes: SHAPES },
	{ key: 'ocean_depth_m', min: 50, max: 2000, step: 25, fallback: 500, places: 0, shapes: SHAPES },
	{ key: 'continent_scale', min: 0.6, max: 4, step: 0.1, fallback: 1.6, places: 1, shapes: ['generated'] },
	{ key: 'sea_share', min: 0.2, max: 0.9, step: 0.01, fallback: 0.55, places: 2, shapes: ['generated'] },
	{ key: 'sea_level_m', min: -400, max: 400, step: 10, fallback: 0, places: 0, shapes: ['earth'] },
	{ key: 'sea_curve', min: 0.25, max: 1, step: 0.05, fallback: 0.45, places: 2, shapes: ['earth'] }
];

/** What a shape's knobs are set to. Only its own are present. */
export type Knobs = Partial<Record<KnobKey, number>>;

/** The knobs that mean something for a shape, in reading order. */
export function knobsFor(shape: Shape): readonly Knob[] {
	return KNOBS.filter((knob) => knob.shapes.includes(shape));
}

/** A value inside the knob's range, rounded to its step. Never NaN. */
export function clampKnob(knob: Knob, value: unknown): number {
	const n = Number(value);
	if (!Number.isFinite(n)) return knob.fallback;
	const stepped = Math.round(n / knob.step) * knob.step;
	return Number(Math.min(knob.max, Math.max(knob.min, stepped)).toFixed(knob.places));
}

/** The params of a recipe: the knobs that this shape uses, and its source. */
export function buildParams(
	shape: Shape,
	values: Knobs,
	field: Field | null
): Record<string, unknown> {
	const params: Record<string, unknown> = {};
	for (const knob of knobsFor(shape)) {
		const value = values[knob.key];
		// A knob left where it was says nothing the generator does not already
		// know, and a recipe should carry only what someone chose.
		if (value !== undefined && value !== knob.fallback) params[knob.key] = value;
	}
	if (field) params.source = { field: field.id };
	return params;
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
