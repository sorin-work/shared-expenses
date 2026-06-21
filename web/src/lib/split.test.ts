import { describe, expect, it } from 'vitest';
import { evenPercentages, formatPercent, parsePercent, previewSplit, type SplitRow } from './split';

const rows = (...values: number[]): SplitRow[] =>
	values.map((value, i) => ({ memberId: `m${i + 1}`, value }));

function amounts(result: ReturnType<typeof previewSplit>): number[] {
	if (!result.ok) throw new Error(result.error);
	return [...result.amounts.values()];
}

describe('previewSplit', () => {
	it('splits equally with leftover cents to the first people', () => {
		expect(amounts(previewSplit(1000, 'equal', rows(0, 0, 0)))).toEqual([334, 333, 333]);
		expect(amounts(previewSplit(1001, 'equal', rows(0, 0, 0)))).toEqual([334, 334, 333]);
	});

	it('uses exact amounts when they add up', () => {
		expect(amounts(previewSplit(5000, 'exact', rows(1250, 3750)))).toEqual([1250, 3750]);
	});

	it('reports how much is left for exact splits', () => {
		const result = previewSplit(5000, 'exact', rows(1250, 3000));
		expect(result).toMatchObject({ ok: false, remaining: 750 });
	});

	it('gives percentage leftovers to the biggest remainder', () => {
		expect(amounts(previewSplit(99, 'percent', rows(1000, 9000)))).toEqual([10, 89]);
		expect(amounts(previewSplit(1000, 'percent', rows(3333, 3333, 3334)))).toEqual([333, 333, 334]);
	});

	it('requires percentages to total 100', () => {
		expect(previewSplit(1000, 'percent', rows(5000, 4000))).toMatchObject({
			ok: false,
			remaining: 1000
		});
	});

	it('splits by shares proportionally', () => {
		expect(amounts(previewSplit(9000, 'shares', rows(2, 1)))).toEqual([6000, 3000]);
		expect(amounts(previewSplit(1000, 'shares', rows(3, 3, 1)))).toEqual([429, 428, 143]);
	});

	it('matches the api for large amounts', () => {
		const result = amounts(previewSplit(Number.MAX_SAFE_INTEGER, 'shares', rows(1, 2)));
		expect(result.reduce((a, b) => a + b, 0)).toBe(Number.MAX_SAFE_INTEGER);
	});

	it('rejects missing amounts, people and values', () => {
		expect(previewSplit(0, 'equal', rows(0)).ok).toBe(false);
		expect(previewSplit(100, 'equal', []).ok).toBe(false);
		expect(previewSplit(100, 'shares', rows(1, 0)).ok).toBe(false);
	});
});

describe('percent helpers', () => {
	it('parses percentages into basis points', () => {
		expect(parsePercent('50')).toBe(5000);
		expect(parsePercent('33.33')).toBe(3333);
		expect(parsePercent('12,5')).toBe(1250);
		expect(parsePercent('7 %')).toBe(700);
		expect(parsePercent('100')).toBe(10000);
	});

	it('rejects invalid percentages', () => {
		expect(parsePercent('')).toBeNull();
		expect(parsePercent('101')).toBeNull();
		expect(parsePercent('1.234')).toBeNull();
		expect(parsePercent('abc')).toBeNull();
	});

	it('formats basis points', () => {
		expect(formatPercent(5000)).toBe('50');
		expect(formatPercent(3333)).toBe('33.33');
		expect(formatPercent(1250)).toBe('12.50');
	});

	it('spreads 100% evenly', () => {
		expect(evenPercentages(3)).toEqual([3334, 3333, 3333]);
		expect(evenPercentages(4)).toEqual([2500, 2500, 2500, 2500]);
		expect(evenPercentages(0)).toEqual([]);
	});
});
