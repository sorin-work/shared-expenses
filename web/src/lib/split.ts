// A client-side copy of the API's split rules, used to preview shares while
// the form is being filled in. The API remains the source of truth.

import type { SplitMethod } from './types';

export const PERCENT_TOTAL = 10_000;

export interface SplitRow {
	memberId: string;
	/** Exact minor units, basis points or share count; ignored for equal. */
	value: number;
}

type SplitOk = { ok: true; amounts: Map<string, number> };
type SplitFailed = { ok: false; error: string; remaining?: number };
export type SplitResult = SplitOk | SplitFailed;

/**
 * Largest remainder allocation, ties to the earlier row. Matches
 * `allocate` in the API so the preview shows the same cents.
 */
function allocate(total: number, weights: number[]): number[] {
	const weightSum = weights.reduce((a, b) => a + b, 0);
	const amounts: number[] = [];
	const remainders: { rem: number; index: number }[] = [];
	weights.forEach((weight, index) => {
		// BigInt keeps total * weight exact for large amounts.
		const exact = BigInt(total) * BigInt(weight);
		amounts.push(Number(exact / BigInt(weightSum)));
		remainders.push({ rem: Number(exact % BigInt(weightSum)), index });
	});
	let leftover = total - amounts.reduce((a, b) => a + b, 0);
	remainders.sort((a, b) => b.rem - a.rem || a.index - b.index);
	for (const { index } of remainders) {
		if (leftover <= 0) break;
		amounts[index] += 1;
		leftover -= 1;
	}
	return amounts;
}

/** Rows must already be in the group's member order. */
export function previewSplit(total: number, method: SplitMethod, rows: SplitRow[]): SplitResult {
	if (!Number.isSafeInteger(total) || total <= 0) {
		return { ok: false, error: 'Enter an amount first' };
	}
	if (rows.length === 0) {
		return { ok: false, error: 'Pick at least one person' };
	}
	if (method !== 'equal' && rows.some((r) => !Number.isSafeInteger(r.value) || r.value <= 0)) {
		return { ok: false, error: 'Every value must be greater than zero' };
	}

	let amounts: number[];
	const values = rows.map((r) => r.value);
	const sum = values.reduce((a, b) => a + b, 0);
	switch (method) {
		case 'equal':
			amounts = allocate(total, new Array<number>(rows.length).fill(1));
			break;
		case 'exact':
			if (sum !== total) {
				return { ok: false, error: 'The amounts must add up to the total', remaining: total - sum };
			}
			amounts = values;
			break;
		case 'percent':
			if (sum !== PERCENT_TOTAL) {
				return {
					ok: false,
					error: 'The percentages must add up to 100%',
					remaining: PERCENT_TOTAL - sum
				};
			}
			amounts = allocate(total, values);
			break;
		case 'shares':
			amounts = allocate(total, values);
			break;
	}
	return { ok: true, amounts: new Map(rows.map((r, i) => [r.memberId, amounts[i]])) };
}

/** Parses "33.33" into basis points (3333). Returns null if invalid. */
export function parsePercent(input: string): number | null {
	const match = /^(\d{1,3})(?:[.,](\d{0,2}))?$/.exec(input.trim().replace(/%$/, '').trim());
	if (!match) return null;
	const [, whole, fraction = ''] = match;
	const bp = Number(whole) * 100 + Number(fraction.padEnd(2, '0'));
	return bp <= PERCENT_TOTAL ? bp : null;
}

export function formatPercent(basisPoints: number): string {
	const whole = Math.floor(basisPoints / 100);
	const fraction = basisPoints % 100;
	return fraction === 0 ? `${whole}` : `${whole}.${String(fraction).padStart(2, '0')}`;
}

/** Spreads 100% over n people in basis points, e.g. 3 -> 3334/3333/3333. */
export function evenPercentages(n: number): number[] {
	if (n <= 0) return [];
	return allocate(PERCENT_TOTAL, new Array<number>(n).fill(1));
}
