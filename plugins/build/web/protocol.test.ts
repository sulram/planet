import { describe, expect, test } from 'bun:test';
import { parseHand, parseHistory, parseOver, parseRefused, parseStroke } from './protocol';

describe('building at the seam', () => {
	test('reads the hand, with a tool or without', () => {
		const look = { finish: 'glass', edge: 'white' };
		expect(parseHand({ type: 'build.hand', tool: 'paint', paint: 4, platform: 16, ...look })).toEqual({ tool: 'paint', paint: 4, platform: 16, ...look });
		expect(parseHand({ tool: null, paint: 0, platform: 8, ...look })).toEqual({ tool: null, paint: 0, platform: 8, ...look });
		expect(parseHand({ tool: 'volume', paint: 0, platform: 8, ...look })?.tool).toBe('volume');
	});

	test('refuses what is no hand', () => {
		const look = { finish: 'matte', edge: 'none' };
		expect(parseHand({ tool: 'hammer', paint: 0, platform: 8, ...look })).toBeNull();
		expect(parseHand({ tool: 'create', paint: '0', platform: 8, ...look })).toBeNull();
		expect(parseHand({ tool: 'create', paint: 0, ...look })).toBeNull();
		expect(parseHand({ tool: 'create', paint: 0, platform: 8, finish: 'wood', edge: 'none' })).toBeNull();
		expect(parseHand({ tool: 'create', paint: 0, platform: 8 })).toBeNull();
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

	test('reads how many cells a stroke covers, and that none is drawn', () => {
		expect(parseStroke({ type: 'build.stroke', size: [12, 1, 5] })).toEqual([12, 1, 5]);
		expect(parseStroke({ size: null })).toBeNull();
		expect(parseStroke({ size: [12, 1] })).toBeNull();
	});
});
