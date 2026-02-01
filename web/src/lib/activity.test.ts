import { describe, expect, it } from 'vitest';
import { activityLink, describeActivity } from './activity';
import type { ActivityEntry } from './types';

function entry(kind: string, payload: Record<string, unknown>, actor = 'Maya'): ActivityEntry {
	return {
		id: 'a1',
		kind,
		payload,
		actor_id: 'u1',
		actor_name: actor,
		created_at: '2026-01-01T10:00:00Z'
	};
}

describe('describeActivity', () => {
	const d = (e: ActivityEntry) => describeActivity(e, 'EUR', 'en-GB');

	it('describes expenses with their amount', () => {
		expect(d(entry('expense_added', { description: 'Rent', amount_cents: 180000 }))).toBe(
			'Maya added Rent (€1,800.00)'
		);
		expect(d(entry('expense_deleted', { description: 'Oops', amount_cents: 500 }))).toBe(
			'Maya deleted Oops (€5.00)'
		);
	});

	it('lists what changed in an edit', () => {
		expect(
			d(
				entry('expense_updated', {
					description: 'Groceries',
					changes: ['amount', 'paid_by', 'split']
				})
			)
		).toBe('Maya edited Groceries (changed amount, payer and split)');
		expect(d(entry('expense_updated', { description: 'Groceries', changes: [] }))).toBe(
			'Maya edited Groceries'
		);
	});

	it('describes settlements', () => {
		expect(d(entry('settlement_added', { from: 'Jonas', to: 'Alex', amount_cents: 2500 }))).toBe(
			'Jonas paid Alex €25.00'
		);
	});

	it('describes membership changes', () => {
		expect(d(entry('member_added', { name: 'Priya' }))).toBe('Maya added Priya');
		expect(d(entry('member_joined', { name: 'Tom' }, 'Tom'))).toBe('Tom joined the group');
		expect(d(entry('member_joined', { name: 'Tom', claimed: true }, 'Thomas'))).toBe(
			'Thomas joined as Tom'
		);
		expect(d(entry('member_removed', { name: 'Tom', left: true }, 'Tom'))).toBe(
			'Tom left the group'
		);
		expect(d(entry('member_removed', { name: 'Tom', left: false }))).toBe('Maya removed Tom');
		expect(d(entry('group_renamed', { from: 'Flat', to: 'Flat 4B' }))).toBe(
			'Maya renamed the group to Flat 4B'
		);
	});

	it('falls back for unknown kinds and deleted users', () => {
		const e = { ...entry('mystery', {}), actor_name: null };
		expect(d(e)).toBe('Someone made a change');
	});
});

describe('activityLink', () => {
	it('links expenses unless they were deleted', () => {
		expect(activityLink(entry('expense_added', { expense_id: 'e1' }), 'g1')).toBe(
			'/groups/g1/expenses/e1'
		);
		expect(activityLink(entry('expense_deleted', { expense_id: 'e1' }), 'g1')).toBeNull();
		expect(activityLink(entry('settlement_added', {}), 'g1')).toBe('/groups/g1/balances');
		expect(activityLink(entry('member_added', {}), 'g1')).toBeNull();
	});
});
