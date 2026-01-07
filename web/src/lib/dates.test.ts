import { describe, expect, it } from 'vitest';
import { formatDate, formatMonth, groupByMonth } from './dates';

describe('dates', () => {
	it('formats dates without shifting the day', () => {
		expect(formatDate('2026-03-01', 'en-GB')).toBe('1 Mar 2026');
		expect(formatDate('2026-12-31T00:00:00Z', 'en-GB')).toBe('31 Dec 2026');
	});

	it('groups by month in order', () => {
		const items = ['2026-03-30', '2026-03-02', '2026-02-14'];
		expect(groupByMonth(items, (d) => d)).toEqual([
			['2026-03', ['2026-03-30', '2026-03-02']],
			['2026-02', ['2026-02-14']]
		]);
		expect(formatMonth('2026-02', 'en-GB')).toBe('February 2026');
	});
});
