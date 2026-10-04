import { describe, expect, test } from 'bun:test';
import { parseHand, parseHistory, parseOver, parseRefused } from './protocol';

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

	test('reads whether a volume stands under the body', () => {
		expect(parseOver({ type: 'build.over', volume: true })).toBe(true);
		expect(parseOver({ volume: false })).toBe(false);
		expect(parseOver({ volume: 'yes' })).toBeNull();
		expect(parseRefused({ reason: 'empty' })).toBe('empty');
	});
});
