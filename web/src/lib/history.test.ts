import { describe, expect, it } from 'vitest';
import { diffExpenses } from './history';
import type { Expense } from './types';

const names: Record<string, string> = { a: 'Alex', b: 'Maya' };
const nameOf = (id: string) => names[id] ?? 'Unknown';

const base: Expense = {
	id: 'e1',
	group_id: 'g1',
	description: 'Groceries',
	amount_cents: 2000,
	paid_by: 'a',
	split_method: 'equal',
	category: 'groceries',
	spent_on: '2026-02-01',
	notes: null,
	created_by: 'u1',
	created_at: '2026-02-01T10:00:00Z',
	updated_at: '2026-02-01T10:00:00Z',
	shares: [
		{ member_id: 'a', input: 1, amount_cents: 1000 },
		{ member_id: 'b', input: 1, amount_cents: 1000 }
	]
};

describe('diffExpenses', () => {
	it('finds nothing between identical versions', () => {
		expect(diffExpenses(base, structuredClone(base), nameOf, 'EUR', 'en-GB')).toEqual([]);
	});

	it('lists changed fields in readable form', () => {
		const after = { ...base, amount_cents: 2450, paid_by: 'b', notes: 'with wine' };
		expect(diffExpenses(base, after, nameOf, 'EUR', 'en-GB')).toEqual([
			{ field: 'Amount', from: '€20.00', to: '€24.50' },
			{ field: 'Paid by', from: 'Alex', to: 'Maya' },
			{ field: 'Notes', from: '—', to: 'with wine' }
		]);
	});

	it('describes split changes', () => {
		const after: Expense = {
			...base,
			split_method: 'percent',
			shares: [
				{ member_id: 'a', input: 6000, amount_cents: 1200 },
				{ member_id: 'b', input: 4000, amount_cents: 800 }
			]
		};
		const [change] = diffExpenses(base, after, nameOf, 'EUR', 'en-GB');
		expect(change).toEqual({
			field: 'Split',
			from: 'equally: Alex, Maya',
			to: 'by percentages: Alex 60%, Maya 40%'
		});
	});
});
