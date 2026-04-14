import { describe, expect, it } from 'vitest';
import { safeNext } from './navigation';

describe('safeNext', () => {
	it('keeps same-site paths', () => {
		expect(safeNext('/groups/1')).toBe('/groups/1');
		expect(safeNext('/join/abc?x=1')).toBe('/join/abc?x=1');
	});

	it('falls back for missing or foreign targets', () => {
		expect(safeNext(null)).toBe('/');
		expect(safeNext('')).toBe('/');
		expect(safeNext('https://evil.example')).toBe('/');
		expect(safeNext('//evil.example')).toBe('/');
		expect(safeNext('/\\evil.example')).toBe('/');
	});
});
