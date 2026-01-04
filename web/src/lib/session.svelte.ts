import type { Session, User } from './types';

const KEY = 'shared-expenses.session';

function load(): Session | null {
	if (typeof localStorage === 'undefined') return null;
	try {
		const raw = localStorage.getItem(KEY);
		return raw ? (JSON.parse(raw) as Session) : null;
	} catch {
		return null;
	}
}

function save(value: Session | null) {
	if (typeof localStorage === 'undefined') return;
	if (value) localStorage.setItem(KEY, JSON.stringify(value));
	else localStorage.removeItem(KEY);
}

class SessionStore {
	current = $state<Session | null>(load());

	get token(): string | null {
		return this.current?.token ?? null;
	}

	get user(): User | null {
		return this.current?.user ?? null;
	}

	signIn(value: Session) {
		this.current = value;
		save(value);
	}

	updateUser(user: User) {
		if (!this.current) return;
		this.current = { ...this.current, user };
		save(this.current);
	}

	signOut() {
		this.current = null;
		save(null);
	}
}

export const session = new SessionStore();
