import { describe, expect, test } from 'bun:test';
import { parseSaid } from './protocol';

describe('parseSaid', () => {
	test('reads a line, with its place or without', () => {
		expect(parseSaid({ type: 'chat.said', session: 2, scope: 'near', text: 'hi', place: '4-K7M42Q' })).toEqual({
			session: 2,
			scope: 'near',
			text: 'hi',
			place: '4-K7M42Q'
		});
		expect(parseSaid({ session: 3, scope: 'world', text: '', place: null })?.place).toBeNull();
	});

	test('refuses what is no line', () => {
		expect(parseSaid({ session: 2, scope: 'everywhere', text: 'hi', place: null })).toBeNull();
		expect(parseSaid({ session: '2', scope: 'near', text: 'hi', place: null })).toBeNull();
		expect(parseSaid({ session: 2, scope: 'near', text: 'hi' })).toBeNull();
	});
});
