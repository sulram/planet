import { describe, expect, test } from 'bun:test';
import { LOW_SIDE, THUMB_SIDE, isPicture, playsFor, within } from './image';

describe('a picture as it is placed', () => {
	test('a version is no longer than its side, the aspect kept', () => {
		expect(within(3000, 2000, LOW_SIDE)).toEqual({ width: 1024, height: 683 });
		expect(within(2000, 3000, THUMB_SIDE)).toEqual({ width: 85, height: 128 });
	});

	test('a small picture is never grown', () => {
		expect(within(64, 32, THUMB_SIDE)).toEqual({ width: 64, height: 32 });
		expect(within(1024, 1024, LOW_SIDE)).toEqual({ width: 1024, height: 1024 });
	});

	test('a side never falls to nothing', () => {
		expect(within(10000, 1, THUMB_SIDE)).toEqual({ width: 128, height: 1 });
	});

	test('png, jpeg, webp and gif are pictures, and an mp4 is one that moves', () => {
		expect(isPicture(new Blob([], { type: 'image/png' }))).toBe(true);
		expect(isPicture(new Blob([], { type: 'image/webp' }))).toBe(true);
		expect(isPicture(new Blob([], { type: 'image/gif' }))).toBe(true);
		expect(isPicture(new Blob([], { type: 'video/mp4' }))).toBe(true);
		expect(isPicture(new Blob([], { type: 'image/svg+xml' }))).toBe(false);
	});

	test('a picture plays for the sum of its frames, and a still for nothing', () => {
		expect(playsFor(1, [])).toBe(0);
		expect(playsFor(3, [40_000, 40_000, 20_000])).toBe(100);
		// A frame with no delay, or under 20 ms, plays for 100 ms, as in every browser.
		expect(playsFor(2, [null, 10_000])).toBe(200);
	});
});
