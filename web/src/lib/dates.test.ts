import { describe, expect, it } from 'vitest';
import { formatDate } from './dates';

describe('dates', () => {
	it('formats dates without shifting the day', () => {
		expect(formatDate('2026-03-01', 'en-GB')).toBe('1 Mar 2026');
		expect(formatDate('2026-12-31T00:00:00Z', 'en-GB')).toBe('31 Dec 2026');
	});
});
