<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { invites } from '$lib/client';
	import { errorMessage } from '$lib/errors';
	import type { InvitePreview } from '$lib/types';

	const code = $derived(page.params.code!);
	let preview = $state<InvitePreview | null>(null);
	let error = $state('');
	let claim = $state<string>('');
	let busy = $state(false);

	$effect(() => {
		invites
			.preview(code)
			.then((p) => {
				if (p.already_member) goto(`/groups/${p.group_id}`, { replaceState: true });
				else preview = p;
			})
			.catch((err) => (error = errorMessage(err)));
	});

	async function join() {
		busy = true;
		try {
			const joined = await invites.join(code, claim || null);
			await goto(`/groups/${joined.group_id}`);
		} catch (err) {
			error = errorMessage(err);
			busy = false;
		}
	}
</script>

<svelte:head><title>Join a group · Shared expenses</title></svelte:head>

<div class="card join">
	{#if error}
		<h1>Can't join</h1>
		<p>
			{error === 'Invite not found' ? 'This invite link is invalid or has been replaced.' : error}
		</p>
		<a href="/">Go to your groups</a>
	{:else if !preview}
		<p class="muted">Loading…</p>
	{:else}
		<h1>Join {preview.name}</h1>
		<p class="muted">
			{preview.member_count}
			{preview.member_count === 1 ? 'person' : 'people'} · expenses in {preview.currency}
		</p>
		{#if preview.placeholders.length}
			<fieldset>
				<legend>Are you one of these people?</legend>
				<p class="muted small">
					Someone already added you without an account. Pick your name to take over your expenses.
				</p>
				<label class="option">
					<input type="radio" name="claim" value="" bind:group={claim} />
					No, join as myself
				</label>
				{#each preview.placeholders as p (p.id)}
					<label class="option">
						<input type="radio" name="claim" value={p.id} bind:group={claim} />
						I'm {p.display_name}
					</label>
				{/each}
			</fieldset>
		{/if}
		<button class="primary" onclick={join} disabled={busy}>
			{busy ? 'Joining…' : 'Join group'}
		</button>
	{/if}
</div>

<style>
	.join {
		max-width: 460px;
		margin: 2rem auto;
	}

	fieldset {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		margin: 1rem 0;
		padding: 0.75rem 1rem;
	}

	.option {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		font-weight: 400;
		margin: 0.4rem 0;
	}
</style>
