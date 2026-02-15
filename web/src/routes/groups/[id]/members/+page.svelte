<script lang="ts">
	import { page } from '$app/state';
	import { groups, members } from '$lib/client';
	import Avatar from '$lib/components/Avatar.svelte';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { useGroup } from '$lib/group.svelte';
	import type { Member } from '$lib/types';

	const ctx = useGroup();
	const group = $derived(ctx.group!);
	const inviteUrl = $derived(`${page.url.origin}/join/${group.invite_code}`);

	let newName = $state('');
	let addError = $state('');
	let error = $state('');
	let copied = $state(false);
	let editing = $state<string | null>(null);
	let editName = $state('');
	let editError = $state('');

	const current = $derived(group.members.filter((m) => !m.left_at));

	async function copy() {
		try {
			await navigator.clipboard.writeText(inviteUrl);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			error = "Couldn't copy the link. Select it and copy it by hand.";
		}
	}

	async function regenerate() {
		if (!confirm('Make a new invite link? The old one will stop working.')) return;
		try {
			await groups.regenerateInvite(group.id);
			await ctx.reload();
		} catch (err) {
			error = errorMessage(err);
		}
	}

	async function add(event: SubmitEvent) {
		event.preventDefault();
		addError = '';
		try {
			await members.add(group.id, newName);
			newName = '';
			await ctx.reload();
		} catch (err) {
			addError = fieldErrors(err).display_name ?? errorMessage(err);
		}
	}

	function startEdit(m: Member) {
		editing = m.id;
		editName = m.display_name;
		editError = '';
	}

	async function saveEdit(event: SubmitEvent) {
		event.preventDefault();
		if (!editing) return;
		try {
			await members.rename(group.id, editing, editName);
			editing = null;
			await ctx.reload();
		} catch (err) {
			editError = fieldErrors(err).display_name ?? errorMessage(err);
		}
	}

	const canRename = (m: Member) => ctx.isOwner || !m.has_account || m.id === group.my_member_id;
</script>

<section class="card">
	<h2>Invite link</h2>
	<p class="muted small">Anyone with this link can join the group after signing in.</p>
	<div class="row">
		<input
			class="invite"
			readonly
			value={inviteUrl}
			aria-label="Invite link"
			onfocus={(e) => e.currentTarget.select()}
		/>
		<button onclick={copy}>{copied ? 'Copied!' : 'Copy'}</button>
		{#if ctx.isOwner}
			<button class="link-button small" onclick={regenerate}>New link</button>
		{/if}
	</div>
</section>

{#if error}<p class="alert" role="alert">{error}</p>{/if}

<section class="card">
	<h2>People</h2>
	<ul class="members">
		{#each current as m (m.id)}
			<li>
				<Avatar name={m.display_name} />
				{#if editing === m.id}
					<form class="row edit" onsubmit={saveEdit}>
						<input bind:value={editName} aria-label="Name" maxlength="60" />
						<button class="primary small" type="submit">Save</button>
						<button class="small" type="button" onclick={() => (editing = null)}>Cancel</button>
						{#if editError}<p class="field-error">{editError}</p>{/if}
					</form>
				{:else}
					<div class="who">
						<strong>{m.display_name}</strong>
						{#if m.id === group.my_member_id}<span class="muted">(you)</span>{/if}
						<div class="muted small">
							{m.role === 'owner'
								? 'Owner'
								: m.has_account
									? 'Member'
									: 'Placeholder, no account yet'}
						</div>
					</div>
					{#if canRename(m)}
						<button class="link-button small" onclick={() => startEdit(m)}>Rename</button>
					{/if}
				{/if}
			</li>
		{/each}
	</ul>

	<form class="add" onsubmit={add} novalidate>
		<label for="new-member">Add someone without an account</label>
		<div class="row">
			<input id="new-member" bind:value={newName} placeholder="Name" maxlength="60" />
			<button type="submit">Add</button>
		</div>
		{#if addError}<p class="field-error">{addError}</p>{/if}
		<p class="muted small">They can claim their place later when they join with the invite link.</p>
	</form>
</section>

<style>
	section + section,
	.alert {
		margin-top: 1rem;
	}

	.invite {
		flex: 1;
		min-width: 14rem;
		font-family: ui-monospace, monospace;
		font-size: 0.9rem;
	}

	.members {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.members li {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.55rem 0;
		border-bottom: 1px solid var(--border);
	}

	.who {
		flex: 1;
	}

	.edit {
		flex: 1;
	}

	.edit input {
		flex: 1;
		min-width: 10rem;
	}

	.add {
		margin-top: 1.25rem;
	}

	.add input {
		flex: 1;
	}
</style>
