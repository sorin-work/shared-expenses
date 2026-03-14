<script lang="ts">
	import { groups } from '$lib/client';
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
{/if}

<style>
	form .row {
		margin-top: 1rem;
	}

	.alert {
		margin-bottom: 1rem;
	}
</style>
