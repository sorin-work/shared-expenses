// Typed calls to the API, using the signed-in session.

import { query, request, type ApiOptions } from './api';
import { session } from './session.svelte';
import type {
	Balances,
	Expense,
	ExpenseInput,
	ExpensePage,
	GroupDetail,
	GroupSummary,
	Revision,
	Session,
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
	balances: (id: string) => call<Balances>(`${g(id)}/balances`)
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
