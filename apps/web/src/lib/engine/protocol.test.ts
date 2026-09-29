import { describe, expect, test } from 'bun:test';
import { parseEvent, type Effects } from './protocol';

describe('parseEvent', () => {
	test('parses the known events', () => {
		expect(parseEvent('{"type":"ready","generator_version":1}')).toEqual({ type: 'ready', generator_version: 1 });
		expect(parseEvent('{"type":"mode_changed","mode":"fly"}')).toEqual({ type: 'mode_changed', mode: 'fly' });
		const effects: Effects = {
			shadows: true,
			grass: true,
			clouds: false,
			cloud_cover: 0.5,
			cloud_density: 1,
			wind_m_s: 14,
			cloud_change: 1,
			exposure: 1,
			bloom: 0.5,
			bloom_threshold: 1.1,
			haze: 1,
			water_clarity: 3,
			tone_map: 'aces'
		};
		expect(parseEvent(JSON.stringify({ type: 'effects_changed', effects }))).toEqual({ type: 'effects_changed', effects });
		expect(parseEvent('{"type":"session","status":"online","session":3}')).toEqual({ type: 'session', status: 'online', session: 3 });
		expect(parseEvent('{"type":"peers","peers":[{"session":2,"name":"Ada","visitor":false}]}')).toEqual({
			type: 'peers',
			peers: [{ session: 2, name: 'Ada', visitor: false }]
		});
		expect(parseEvent('{"type":"said","session":2,"scope":"near","text":"hi","place":"4-K7M42Q"}')).toEqual({
			type: 'said',
			session: 2,
			scope: 'near',
			text: 'hi',
			place: '4-K7M42Q'
		});
		expect(parseEvent('{"type":"anchors","anchors":[{"session":2,"x":0.5,"y":0.4,"distance_m":3}]}')).toEqual({
			type: 'anchors',
			anchors: [{ session: 2, x: 0.5, y: 0.4, distance_m: 3 }]
		});
		expect(parseEvent('{"type":"tool_changed","tool":"paint","paint":4,"platform":16}')).toEqual({ type: 'tool_changed', tool: 'paint', paint: 4, platform: 16 });
		expect(parseEvent('{"type":"tool_changed","tool":null,"paint":0,"platform":8}')).toEqual({ type: 'tool_changed', tool: null, paint: 0, platform: 8 });
		expect(parseEvent('{"type":"palette","colors":["#f2f0eb"]}')).toEqual({ type: 'palette', colors: ['#f2f0eb'] });
		expect(parseEvent('{"type":"build_refused","reason":"sea"}')).toEqual({ type: 'build_refused', reason: 'sea' });
		expect(parseEvent('{"type":"history","undo":true,"redo":false}')).toEqual({ type: 'history', undo: true, redo: false });
	});

	test('ignores unknown types and malformed payloads', () => {
		expect(parseEvent('{"type":"from_the_future"}')).toBeNull();
		expect(parseEvent('{"no_type":true}')).toBeNull();
		expect(parseEvent('null')).toBeNull();
		expect(parseEvent('not json')).toBeNull();
	});
});
