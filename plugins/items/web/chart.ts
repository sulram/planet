import type { LandmarkOne } from './protocol';

/**
 * The chart's search, by name: what was typed, held anywhere in a name,
 * whatever its case and its accents. Over the landmarks the world answered,
 * in their order. Pure: no WASM, no DOM.
 */

/** A name as the search compares it: no case, no accents. */
export function fold(text: string): string {
	return text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
}

/** The landmarks whose name holds what was typed, in the order given; all of them for nothing typed. */
export function search(landmarks: LandmarkOne[], typed: string): LandmarkOne[] {
	const wanted = fold(typed.trim());
	return wanted ? landmarks.filter((one) => fold(one.name).includes(wanted)) : landmarks;
}
