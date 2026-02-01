// Shapes of the API responses. Amounts are integers in the currency's minor
// unit (cents), dates are ISO strings.

export interface User {
	id: string;
	email: string;
	name: string;
	created_at: string;
}

export interface Session {
	token: string;
	user: User;
}

export type Role = 'owner' | 'member';

export interface Member {
	id: string;
	display_name: string;
	role: Role;
	has_account: boolean;
	joined_at: string;
	left_at: string | null;
}

export interface GroupSummary {
	id: string;
	name: string;
	currency: string;
	member_count: number;
	my_member_id: string;
	my_balance: number;
	created_at: string;
}

export interface GroupDetail {
	id: string;
	name: string;
	currency: string;
	invite_code: string;
	created_at: string;
	my_member_id: string;
	my_role: Role;
	members: Member[];
}

export type SplitMethod = 'equal' | 'exact' | 'percent' | 'shares';

export interface ExpenseShare {
	member_id: string;
	input: number;
	amount_cents: number;
}

export interface Expense {
	id: string;
	group_id: string;
	description: string;
	amount_cents: number;
	paid_by: string;
	split_method: SplitMethod;
	category: string;
	spent_on: string;
	notes: string | null;
	created_by: string;
	created_at: string;
	updated_at: string;
	shares: ExpenseShare[];
}

export interface ExpensePage {
	items: Expense[];
	total: number;
}

export interface ExpenseInput {
	description: string;
	amount_cents: number;
	paid_by: string;
	split_method: SplitMethod;
	category: string;
	spent_on: string;
	notes: string | null;
	splits: { member_id: string; value: number }[];
}

export interface Revision {
	revision: number;
	action: 'created' | 'updated' | 'deleted';
	snapshot: Expense;
	changed_by: string | null;
	changed_by_name: string | null;
	changed_at: string;
}

export interface Settlement {
	id: string;
	from_member: string;
	to_member: string;
	amount_cents: number;
	settled_on: string;
	note: string | null;
	created_by: string;
	created_at: string;
}

export interface BalanceRow {
	member_id: string;
	display_name: string;
	left: boolean;
	paid: number;
	share: number;
	sent: number;
	received: number;
	net: number;
}

export interface Suggestion {
	from_member: string;
	to_member: string;
	amount_cents: number;
}

export interface Balances {
	currency: string;
	balances: BalanceRow[];
	suggestions: Suggestion[];
}

export interface ActivityEntry {
	id: string;
	kind: string;
	payload: Record<string, unknown>;
	actor_id: string | null;
	actor_name: string | null;
	created_at: string;
}

export interface ActivityFeed {
	items: ActivityEntry[];
	next_before: string | null;
}
