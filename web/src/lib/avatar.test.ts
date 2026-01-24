import { describe, expect, it } from 'vitest';
import { hue, initials } from './avatar';

describe('initials', () => {
	it('uses the first and last name', () => {
		expect(initials('Maya Lopez')).toBe('ML');
		expect(initials('Anna Maria de la Cruz')).toBe('AC');
		expect(initials('jonas')).toBe('J');
		expect(initials('  ')).toBe('?');
	});
});

describe('hue', () => {
	it('is stable and in range', () => {
		expect(hue('Maya')).toBe(hue('Maya'));
		for (const name of ['Alex', 'Sam', 'Priya', '']) {
			expect(hue(name)).toBeGreaterThanOrEqual(0);
			expect(hue(name)).toBeLessThan(360);
		}
	});
});
