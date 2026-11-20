import { describe, expect, test } from 'bun:test';
import { search } from './chart';
import { parseLandmarks, type LandmarkOne } from './protocol';

const one = (id: number, name: string, home = false, favourite = false): LandmarkOne => ({
	id,
	name,
	paint: 0,
	away_m: id * 10,
	place: `4-K7M42${id}`,
	home,
	favourite
});

describe('search', () => {
	const chart = [one(1, 'Praça da Sé', true), one(2, 'São Paulo', false, true), one(3, 'Rua Augusta'), one(4, '天安门')];

	test('holds what was typed, whatever its case and accents, in the order given', () => {
		expect(search(chart, 'sa').map((one) => one.id)).toEqual([2]);
		expect(search(chart, 'PRACA').map((one) => one.id)).toEqual([1]);
		expect(search(chart, 'sé').map((one) => one.id)).toEqual([1]);
		expect(search(chart, 'a').map((one) => one.id)).toEqual([1, 2, 3]);
		expect(search(chart, '天安').map((one) => one.id)).toEqual([4]);
	});

	test('nothing typed is the whole chart; nothing held, none', () => {
		expect(search(chart, '  ')).toEqual(chart);
		expect(search(chart, 'zz')).toEqual([]);
	});
});

describe('parseLandmarks', () => {
	test('reads whether a landmark is the home and a favourite', () => {
		const event = { landmarks: [{ id: 7, name: 'Sé', paint: 3, away_m: 12, place: '4-K7M42Q', home: true, favourite: false }] };
		expect(parseLandmarks(event)).toEqual([{ id: 7, name: 'Sé', paint: 3, away_m: 12, place: '4-K7M42Q', home: true, favourite: false }]);
		const older = { landmarks: [{ id: 7, name: 'Sé', paint: 3, away_m: null, place: '4-K7M42Q' }] };
		expect(parseLandmarks(older)?.[0]).toMatchObject({ away_m: null, home: false, favourite: false });
	});
});
