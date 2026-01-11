// Amounts travel as integers in the currency's minor unit. These helpers
// convert between that and what people type and read.

const ZERO_DECIMAL = new Set(['JPY', 'KRW']);

export const CURRENCIES = [
	'AUD',
	'BRL',
	'CAD',
	'CHF',
	'CZK',
	'DKK',
	'EUR',
	'GBP',
	'HUF',
	'INR',
	'JPY',
	'KRW',
	'MXN',
	'NOK',
	'NZD',
	'PLN',
	'RON',
	'SEK',
	'USD',
	'ZAR'
] as const;

export function minorUnits(currency: string): number {
	return ZERO_DECIMAL.has(currency.toUpperCase()) ? 0 : 2;
}

/**
 * Parses an amount typed by a person into minor units. Accepts "12", "12.5",
 * "1,234.56" and a leading currency symbol. Returns null for
 * anything else, including more decimals than the currency has.
 */
export function parseAmount(input: string, currency: string): number | null {
	const decimals = minorUnits(currency);
	const cleaned = input
		.trim()
		.replace(/^[^\d.,-]+/, '')
		.replace(/[\s']/g, '');
	if (cleaned === '') return null;

	const normalised = cleaned.replace(/,/g, '');

	const match = /^(\d+)(?:\.(\d*))?$/.exec(normalised);
	if (!match) return null;
	const [, whole, fraction = ''] = match;
	if (fraction.length > decimals) return null;

	const minor = Number(whole) * 10 ** decimals + Number(fraction.padEnd(decimals, '0') || 0);
	return Number.isSafeInteger(minor) ? minor : null;
}

/** "12.50" style string for an input field, without currency symbol. */
export function toInputValue(minor: number, currency: string): string {
	const decimals = minorUnits(currency);
	if (decimals === 0) return String(minor);
	const sign = minor < 0 ? '-' : '';
	const abs = Math.abs(minor);
	const factor = 10 ** decimals;
	return `${sign}${Math.floor(abs / factor)}.${String(abs % factor).padStart(decimals, '0')}`;
}

/** Formats minor units for display, e.g. 1234 EUR -> "€12.34". */
export function formatMoney(minor: number, currency: string, locale?: string): string {
	const decimals = minorUnits(currency);
	return new Intl.NumberFormat(locale, {
		style: 'currency',
		currency,
		minimumFractionDigits: decimals,
		maximumFractionDigits: decimals
	}).format(minor / 10 ** decimals);
}
