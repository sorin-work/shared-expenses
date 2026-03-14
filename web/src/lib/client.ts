// Typed calls to the API, using the signed-in session.

import { query, rawRequest, request, type ApiOptions } from './api';
import { session } from './session.svelte';
import type {
	ActivityFeed,
	Balances,
	Expense,
	ExpenseInput,
	ExpensePage,
	GroupDetail,
	GroupSummary,
	InvitePreview,
	Member,
	Revision,
	Session,
	Settlement,
	User
} from './types';

function call<T>(path: string, options: Omit<ApiOptions, 'token'> = {}) {
	return request<T>(path, { ...options, token: session.token });
}

const g = (groupId: string) => `/api/groups/${groupId}`;

export const auth = {
	login: (email: string, password: string) =>
		request<Session>('/api/auth/login', { method: 'POST', body: { email, password } }),
	signup: (email: string, name: string, password: string) =>
		request<Session>('/api/auth/signup', { method: 'POST', body: { email, name, password } }),
	me: () => call<User>('/api/auth/me')
};

export const groups = {
	list: () => call<GroupSummary[]>('/api/groups'),
	create: (name: string, currency: string) =>
		call<GroupDetail>('/api/groups', { method: 'POST', body: { name, currency } }),
	get: (id: string) => call<GroupDetail>(g(id)),
	update: (id: string, changes: { name?: string; currency?: string }) =>
		call<GroupDetail>(g(id), { method: 'PATCH', body: changes }),
	remove: (id: string) => call<void>(g(id), { method: 'DELETE' }),
	regenerateInvite: (id: string) =>
		call<{ invite_code: string }>(`${g(id)}/invite`, { method: 'POST' }),
	balances: (id: string) => call<Balances>(`${g(id)}/balances`),
	activity: (id: string, before?: string | null) =>
		call<ActivityFeed>(`${g(id)}/activity${query({ before, limit: 30 })}`),
	/** Downloads the CSV export through the API so the token is sent. */
	async exportCsv(id: string): Promise<{ blob: Blob; filename: string }> {
		const response = await rawRequest(`${g(id)}/export.csv`, { token: session.token });
		const disposition = response.headers.get('Content-Disposition') ?? '';
		const filename = /filename="([^"]+)"/.exec(disposition)?.[1] ?? 'expenses.csv';
		return { blob: await response.blob(), filename };
	}
};

export const members = {
	add: (groupId: string, display_name: string) =>
		call<Member>(`${g(groupId)}/members`, { method: 'POST', body: { display_name } }),
	rename: (groupId: string, memberId: string, display_name: string) =>
		call<Member>(`${g(groupId)}/members/${memberId}`, { method: 'PATCH', body: { display_name } })
};

export interface ExpenseFilters {
	q?: string;
	category?: string;
	member?: string;
	offset?: number;
	limit?: number;
}

export const expenses = {
	list: (groupId: string, filters: ExpenseFilters = {}) =>
		call<ExpensePage>(`${g(groupId)}/expenses${query({ ...filters })}`),
	get: (groupId: string, id: string) => call<Expense>(`${g(groupId)}/expenses/${id}`),
	create: (groupId: string, input: ExpenseInput) =>
		call<Expense>(`${g(groupId)}/expenses`, { method: 'POST', body: input }),
	update: (groupId: string, id: string, input: ExpenseInput) =>
		call<Expense>(`${g(groupId)}/expenses/${id}`, { method: 'PUT', body: input }),
	remove: (groupId: string, id: string) =>
		call<void>(`${g(groupId)}/expenses/${id}`, { method: 'DELETE' }),
	history: (groupId: string, id: string) => call<Revision[]>(`${g(groupId)}/expenses/${id}/history`)
};

export const settlements = {
	list: (groupId: string) => call<Settlement[]>(`${g(groupId)}/settlements`),
	create: (
		groupId: string,
		input: {
			from_member: string;
			to_member: string;
			amount_cents: number;
			settled_on: string;
			note: string | null;
		}
	) => call<Settlement>(`${g(groupId)}/settlements`, { method: 'POST', body: input }),
	remove: (groupId: string, id: string) =>
		call<void>(`${g(groupId)}/settlements/${id}`, { method: 'DELETE' })
};

export const invites = {
	preview: (code: string) => call<InvitePreview>(`/api/invites/${encodeURIComponent(code)}`),
	join: (code: string, claim_member_id: string | null) =>
		call<{ group_id: string; member_id: string }>(`/api/invites/${encodeURIComponent(code)}/join`, {
			method: 'POST',
			body: { claim_member_id }
		})
};
