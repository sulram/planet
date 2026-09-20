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
	});

	test('ignores unknown types and malformed payloads', () => {
		expect(parseEvent('{"type":"from_the_future"}')).toBeNull();
		expect(parseEvent('{"no_type":true}')).toBeNull();
		expect(parseEvent('null')).toBeNull();
		expect(parseEvent('not json')).toBeNull();
	});
});
