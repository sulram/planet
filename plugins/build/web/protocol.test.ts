import { describe, expect, test } from 'bun:test';
import { parseHand, parseHistory, parseRefused } from './protocol';

describe('building at the seam', () => {
	test('reads the hand, with a tool or without', () => {
		expect(parseHand({ type: 'build.hand', tool: 'paint', paint: 4, platform: 16 })).toEqual({ tool: 'paint', paint: 4, platform: 16 });
		expect(parseHand({ tool: null, paint: 0, platform: 8 })).toEqual({ tool: null, paint: 0, platform: 8 });
	});

	test('refuses what is no hand', () => {
		expect(parseHand({ tool: 'hammer', paint: 0, platform: 8 })).toBeNull();
		expect(parseHand({ tool: 'create', paint: '0', platform: 8 })).toBeNull();
		expect(parseHand({ tool: 'create', paint: 0 })).toBeNull();
	});

	test('reads why a platform was refused, and what there is to take back', () => {
		expect(parseRefused({ type: 'build.refused', reason: 'sea' })).toBe('sea');
		expect(parseRefused({ reason: 'weather' })).toBeNull();
		expect(parseHistory({ type: 'build.history', undo: true, redo: false })).toEqual({ undo: true, redo: false });
		expect(parseHistory({ undo: 1, redo: 0 })).toBeNull();
	});
});
