// Typed calls to the API, using the signed-in session.

import { request, type ApiOptions } from './api';
import { session } from './session.svelte';
import type { Session, User } from './types';

function call<T>(path: string, options: Omit<ApiOptions, 'token'> = {}) {
	return request<T>(path, { ...options, token: session.token });
}

export const auth = {
	login: (email: string, password: string) =>
		request<Session>('/api/auth/login', { method: 'POST', body: { email, password } }),
	signup: (email: string, name: string, password: string) =>
		request<Session>('/api/auth/signup', { method: 'POST', body: { email, name, password } }),
	me: () => call<User>('/api/auth/me')
};
