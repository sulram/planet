import { describe, expect, test } from 'bun:test';
import { parseEvent } from './protocol';

describe('parseEvent', () => {
	test('parses the known events', () => {
		expect(parseEvent('{"type":"ready","generator_version":1}')).toEqual({ type: 'ready', generator_version: 1 });
		expect(parseEvent('{"type":"mode_changed","mode":"fly"}')).toEqual({ type: 'mode_changed', mode: 'fly' });
	});

	test('ignores unknown types and malformed payloads', () => {
		expect(parseEvent('{"type":"from_the_future"}')).toBeNull();
		expect(parseEvent('{"no_type":true}')).toBeNull();
		expect(parseEvent('null')).toBeNull();
		expect(parseEvent('not json')).toBeNull();
	});
});
