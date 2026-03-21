<script lang="ts">
	import { goto } from '$app/navigation';
	import { groups, members } from '$lib/client';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { useGroup } from '$lib/group.svelte';
	import { CURRENCIES } from '$lib/money';

	const ctx = useGroup();
	const group = $derived(ctx.group!);

	// svelte-ignore state_referenced_locally
	let name = $state(group.name);
	// svelte-ignore state_referenced_locally
	let currency = $state(group.currency);
	let saved = $state(false);
	let error = $state('');
	let fields = $state<Record<string, string>>({});

	async function save(event: SubmitEvent) {
		event.preventDefault();
		error = '';
		fields = {};
		saved = false;
		try {
			await groups.update(group.id, { name, currency });
			await ctx.reload();
			saved = true;
		} catch (err) {
			error = errorMessage(err);
			fields = fieldErrors(err);
		}
	}

	async function deleteGroup() {
		const answer = prompt(
			`This deletes "${group.name}" with all its expenses for everyone. Type the group name to confirm.`
		);
		if (answer?.trim() !== group.name) return;
		try {
			await groups.remove(group.id);
			await goto('/');
		} catch (err) {
			error = errorMessage(err);
		}
	}

	async function leave() {
		if (!confirm(`Leave "${group.name}"? You need to be settled up first.`)) return;
		try {
			await members.remove(group.id, group.my_member_id);
			await goto('/');
		} catch (err) {
			error = errorMessage(err);
		}
	}
</script>

{#if error}<p class="alert" role="alert">{error}</p>{/if}

{#if ctx.isOwner}
	<form class="card" onsubmit={save} novalidate>
		<h2>Group settings</h2>
		<div class="field">
			<label for="name">Name</label>
			<input id="name" bind:value={name} maxlength="80" aria-invalid={!!fields.name} />
			{#if fields.name}<p class="field-error">{fields.name}</p>{/if}
		</div>
		<div class="field">
			<label for="currency">Currency</label>
			<select id="currency" bind:value={currency} aria-invalid={!!fields.currency}>
				{#each CURRENCIES as code (code)}
					<option value={code}>{code}</option>
				{/each}
			</select>
			<p class="muted small">The currency can only be changed while the group has no expenses.</p>
		</div>
		<div class="row">
			<button class="primary" type="submit">Save</button>
			{#if saved}<span class="positive small">Saved</span>{/if}
		</div>
	</form>

	<section class="card danger-zone">
		<h2>Delete group</h2>
		<p class="muted">Removes the group, its expenses and payments for every member.</p>
		<button class="danger" onclick={deleteGroup}>Delete this group</button>
	</section>
{:else}
	<section class="card">
		<h2>Leave group</h2>
		<p class="muted">
			You can leave once your balance is zero. Your past expenses stay in the group's history.
		</p>
		<button class="danger" onclick={leave}>Leave this group</button>
	</section>
{/if}

<style>
	form .row {
		margin-top: 1rem;
	}

	.danger-zone {
		margin-top: 1rem;
	}

	.alert {
		margin-bottom: 1rem;
	}
</style>
