import { describe, expect, it } from 'vitest';
import { formatDate, formatMonth, groupByMonth, timeAgo, today } from './dates';

describe('dates', () => {
	it('formats today as a local date', () => {
		expect(today(new Date(2026, 0, 5, 23, 30))).toBe('2026-01-05');
	});

	it('formats dates without shifting the day', () => {
		expect(formatDate('2026-03-01', 'en-GB')).toBe('1 Mar 2026');
		expect(formatDate('2026-12-31T00:00:00Z', 'en-GB')).toBe('31 Dec 2026');
	});

	it('describes how long ago something happened', () => {
		const now = new Date('2026-05-10T12:00:00Z');
		expect(timeAgo('2026-05-10T11:59:30Z', now)).toBe('just now');
		expect(timeAgo('2026-05-10T11:15:00Z', now)).toBe('45 min ago');
		expect(timeAgo('2026-05-10T07:00:00Z', now)).toBe('5 h ago');
		expect(timeAgo('2026-05-09T10:00:00Z', now)).toBe('yesterday');
		expect(timeAgo('2026-05-06T12:00:00Z', now)).toBe('4 days ago');
		expect(timeAgo('2026-04-01T12:00:00Z', now, 'en-GB')).toBe('1 Apr 2026');
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
