import { category } from './categories';
import { formatDate } from './dates';
import { formatMoney } from './money';
import { formatPercent } from './split';
import type { Expense, ExpenseShare } from './types';

export interface Change {
	field: string;
	from: string;
	to: string;
}

const METHOD_LABELS: Record<string, string> = {
	equal: 'equally',
	exact: 'by exact amounts',
	percent: 'by percentages',
	shares: 'by shares'
};

export function describeSplit(
	method: string,
	shares: ExpenseShare[],
	nameOf: (memberId: string) => string,
	currency: string,
	locale?: string
): string {
	const people = shares.map((s) => {
		const name = nameOf(s.member_id);
		switch (method) {
			case 'exact':
				return `${name} ${formatMoney(s.amount_cents, currency, locale)}`;
			case 'percent':
				return `${name} ${formatPercent(s.input)}%`;
			case 'shares':
				return `${name} ×${s.input}`;
			default:
				return name;
		}
	});
	return `${METHOD_LABELS[method] ?? method}: ${people.join(', ')}`;
}

/** Human readable differences between two versions of an expense. */
export function diffExpenses(
	before: Expense,
	after: Expense,
	nameOf: (memberId: string) => string,
	currency: string,
	locale?: string
): Change[] {
	const changes: Change[] = [];
	const add = (field: string, from: string, to: string) => {
		if (from !== to) changes.push({ field, from, to });
	};

	add('Description', before.description, after.description);
	add(
		'Amount',
		formatMoney(before.amount_cents, currency, locale),
		formatMoney(after.amount_cents, currency, locale)
	);
	add('Paid by', nameOf(before.paid_by), nameOf(after.paid_by));
	add('Category', category(before.category).label, category(after.category).label);
	add('Date', formatDate(before.spent_on, locale), formatDate(after.spent_on, locale));
	add('Notes', before.notes ?? '—', after.notes ?? '—');
	add(
		'Split',
		describeSplit(before.split_method, before.shares, nameOf, currency, locale),
		describeSplit(after.split_method, after.shares, nameOf, currency, locale)
	);
	return changes;
}
