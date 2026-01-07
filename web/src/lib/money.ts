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
