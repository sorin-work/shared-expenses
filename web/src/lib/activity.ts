import { formatMoney } from './money';
import type { ActivityEntry } from './types';

const FIELD_LABELS: Record<string, string> = {
	description: 'description',
	amount: 'amount',
	paid_by: 'payer',
	category: 'category',
	date: 'date',
	notes: 'notes',
	split: 'split'
};

function str(value: unknown, fallback = 'someone'): string {
	return typeof value === 'string' && value !== '' ? value : fallback;
}

function joinList(items: string[]): string {
	if (items.length <= 1) return items.join('');
	return `${items.slice(0, -1).join(', ')} and ${items[items.length - 1]}`;
}

/** A one line description of a feed entry, e.g. "Maya added Rent (€1,800.00)". */
export function describeActivity(entry: ActivityEntry, currency: string, locale?: string): string {
	const p = entry.payload;
	const actor = entry.actor_name ?? 'Someone';
	const amount =
		typeof p.amount_cents === 'number' ? formatMoney(p.amount_cents, currency, locale) : '';
	const withAmount = (text: string) => (amount ? `${text} (${amount})` : text);

	switch (entry.kind) {
		case 'group_created':
			return `${actor} created the group`;
		case 'group_renamed':
			return `${actor} renamed the group to ${str(p.to, 'something new')}`;
		case 'member_added':
			return `${actor} added ${str(p.name)}`;
		case 'member_joined':
			return p.claimed
				? `${actor} joined as ${str(p.name)}`
				: `${str(p.name, actor)} joined the group`;
		case 'member_renamed':
			return `${actor} renamed ${str(p.from)} to ${str(p.to)}`;
		case 'member_removed':
			return p.left ? `${str(p.name, actor)} left the group` : `${actor} removed ${str(p.name)}`;
		case 'expense_added':
			return withAmount(`${actor} added ${str(p.description, 'an expense')}`);
		case 'expense_updated': {
			const changes = Array.isArray(p.changes)
				? p.changes.map((c) => FIELD_LABELS[String(c)] ?? String(c))
				: [];
			const what = changes.length ? ` (changed ${joinList(changes)})` : '';
			return `${actor} edited ${str(p.description, 'an expense')}${what}`;
		}
		case 'expense_deleted':
			return withAmount(`${actor} deleted ${str(p.description, 'an expense')}`);
		case 'settlement_added':
			return `${str(p.from)} paid ${str(p.to)} ${amount}`.trim();
		case 'settlement_deleted':
			return withAmount(`${actor} removed a payment from ${str(p.from)} to ${str(p.to)}`);
		default:
			return `${actor} made a change`;
	}
}

/** Which page an entry links to, if any. */
export function activityLink(entry: ActivityEntry, groupId: string): string | null {
	const id = entry.payload.expense_id;
	if (typeof id === 'string' && entry.kind !== 'expense_deleted') {
		return `/groups/${groupId}/expenses/${id}`;
	}
	if (entry.kind === 'settlement_added') return `/groups/${groupId}/balances`;
	return null;
}
