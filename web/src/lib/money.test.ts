import { describe, expect, it } from 'vitest';
import { formatMoney, minorUnits, parseAmount, toInputValue } from './money';

describe('parseAmount', () => {
	it('parses whole and decimal amounts', () => {
		expect(parseAmount('12', 'EUR')).toBe(1200);
		expect(parseAmount('12.5', 'EUR')).toBe(1250);
		expect(parseAmount('12.50', 'EUR')).toBe(1250);
		expect(parseAmount('0.05', 'EUR')).toBe(5);
		expect(parseAmount('.5', 'EUR')).toBeNull();
	});

	it('accepts a decimal comma', () => {
		expect(parseAmount('12,50', 'EUR')).toBe(1250);
		expect(parseAmount('3,5', 'EUR')).toBe(350);
	});

	it('ignores thousands separators and spaces', () => {
		expect(parseAmount('1,234.56', 'USD')).toBe(123456);
		expect(parseAmount('1 234.56', 'EUR')).toBe(123456);
		expect(parseAmount('1,234', 'USD')).toBe(123400);
		expect(parseAmount("1'234.50", 'CHF')).toBe(123450);
	});

	it('rejects garbage and too many decimals', () => {
		expect(parseAmount('', 'EUR')).toBeNull();
		expect(parseAmount('abc', 'EUR')).toBeNull();
		expect(parseAmount('1.234', 'EUR')).toBeNull();
		expect(parseAmount('1.2.3', 'EUR')).toBeNull();
		expect(parseAmount('-5', 'EUR')).toBeNull();
		expect(parseAmount('12.5', 'JPY')).toBeNull();
	});

	it('handles zero-decimal currencies', () => {
		expect(parseAmount('1500', 'JPY')).toBe(1500);
		expect(minorUnits('jpy')).toBe(0);
	});
});

describe('toInputValue', () => {
	it('round-trips with parseAmount', () => {
		for (const minor of [0, 5, 99, 100, 123456]) {
			expect(parseAmount(toInputValue(minor, 'EUR'), 'EUR')).toBe(minor);
		}
		expect(toInputValue(1500, 'JPY')).toBe('1500');
		expect(toInputValue(-250, 'EUR')).toBe('-2.50');
	});
});

describe('formatMoney', () => {
	it('formats with the currency symbol', () => {
		expect(formatMoney(1234, 'EUR', 'en-GB')).toBe('€12.34');
		expect(formatMoney(-500, 'USD', 'en-US')).toBe('-$5.00');
		expect(formatMoney(1500, 'JPY', 'en-US')).toBe('¥1,500');
	});
});
