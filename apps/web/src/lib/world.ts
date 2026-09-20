/**
 * Pure world types and seed helpers. No PocketBase, no DOM.
 * Field names match the wire form shared by the engine and the `worlds`
 * collection, so a Recipe travels unmapped.
 */

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
