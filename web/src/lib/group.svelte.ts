import { getContext, setContext } from 'svelte';
import { groups } from './client';
import { errorMessage } from './errors';
import type { GroupDetail, Member } from './types';

/** The group being viewed, shared by all pages under /groups/[id]. */
export class GroupState {
	group = $state<GroupDetail | null>(null);
	error = $state('');
	#loadedId: string | null = null;

	/** Members who can be used in new expenses, in member order. */
	active = $derived<Member[]>(this.group?.members.filter((m) => !m.left_at) ?? []);

	async load(id: string) {
		if (this.#loadedId !== id) {
			this.group = null;
			this.error = '';
		}
		this.#loadedId = id;
		try {
			const group = await groups.get(id);
			if (this.#loadedId === id) this.group = group;
		} catch (err) {
			if (this.#loadedId === id) this.error = errorMessage(err);
		}
	}

	reload() {
		return this.#loadedId ? this.load(this.#loadedId) : Promise.resolve();
	}

	nameOf = (memberId: string): string =>
		this.group?.members.find((m) => m.id === memberId)?.display_name ?? 'Former member';

	get isOwner(): boolean {
		return this.group?.my_role === 'owner';
	}
}

const KEY = Symbol('group');

export function provideGroup(state: GroupState) {
	setContext(KEY, state);
}

export function useGroup(): GroupState {
	return getContext<GroupState>(KEY);
}
