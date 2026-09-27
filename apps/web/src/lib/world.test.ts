import { describe, expect, test } from 'bun:test';
import { fieldUrl, isGeneratorVersion, isSeed, normalizeSeed, randomSeed } from './world';

describe('isSeed', () => {
	test('accepts 16 lowercase hex digits', () => {
		expect(isSeed('00000000deadbeef')).toBe(true);
		expect(isSeed('ffffffffffffffff')).toBe(true);
	});

	test('rejects anything else', () => {
		expect(isSeed('')).toBe(false);
		expect(isSeed('deadbeef')).toBe(false);
		expect(isSeed('00000000DEADBEEF')).toBe(false);
		expect(isSeed('00000000deadbeefa')).toBe(false);
		expect(isSeed('00000000deadbeeg')).toBe(false);
		expect(isSeed(' 0000000deadbeef')).toBe(false);
	});
});

describe('normalizeSeed', () => {
	test('keeps a canonical seed', () => {
		expect(normalizeSeed('00000000deadbeef')).toBe('00000000deadbeef');
	});

	test('trims, lowers, strips 0x and pads', () => {
		expect(normalizeSeed('  DEADBEEF ')).toBe('00000000deadbeef');
		expect(normalizeSeed('0xDeadBeef')).toBe('00000000deadbeef');
		expect(normalizeSeed('0')).toBe('0000000000000000');
	});

	test('returns null for input that is not a u64 in hex', () => {
		expect(normalizeSeed(null)).toBeNull();
		expect(normalizeSeed(undefined)).toBeNull();
		expect(normalizeSeed('')).toBeNull();
		expect(normalizeSeed('0x')).toBeNull();
		expect(normalizeSeed('planet')).toBeNull();
		expect(normalizeSeed('10000000000000000')).toBeNull();
		expect(normalizeSeed('dead beef')).toBeNull();
	});

	test('output always satisfies isSeed', () => {
		for (const raw of ['1', 'ABC', '0xff', 'ffffffffffffffff']) {
			expect(isSeed(normalizeSeed(raw)!)).toBe(true);
		}
	});
});

describe('randomSeed', () => {
	test('always yields a canonical seed', () => {
		for (let i = 0; i < 256; i++) expect(isSeed(randomSeed())).toBe(true);
	});

	test('does not repeat in practice', () => {
		const seen = new Set(Array.from({ length: 256 }, randomSeed));
		expect(seen.size).toBe(256);
	});
});

describe('isGeneratorVersion', () => {
	test('accepts integers from 1', () => {
		expect(isGeneratorVersion(1)).toBe(true);
		expect(isGeneratorVersion(42)).toBe(true);
	});

	test('rejects zero, negatives, fractions and NaN', () => {
		expect(isGeneratorVersion(0)).toBe(false);
		expect(isGeneratorVersion(-1)).toBe(false);
		expect(isGeneratorVersion(1.5)).toBe(false);
		expect(isGeneratorVersion(Number.NaN)).toBe(false);
	});
});

describe('fieldUrl', () => {
	test('carries the content id, so a rebake is a new URL', () => {
		expect(fieldUrl('earth', 'abc123')).toBe('/assets/fields/earth.field?abc123');
	});
});
