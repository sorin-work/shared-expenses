import { describe, expect, it } from 'vitest';
import { formatMoney } from './money';

describe('formatMoney', () => {
	it('formats with the currency symbol', () => {
		expect(formatMoney(1234, 'EUR', 'en-GB')).toBe('€12.34');
		expect(formatMoney(-500, 'USD', 'en-US')).toBe('-$5.00');
		expect(formatMoney(1500, 'JPY', 'en-US')).toBe('¥1,500');
	});
});
