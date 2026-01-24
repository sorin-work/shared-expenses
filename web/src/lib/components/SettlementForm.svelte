<script lang="ts">
	import { settlements } from '$lib/client';
	import { today } from '$lib/dates';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { parseAmount, toInputValue } from '$lib/money';
	import type { GroupDetail } from '$lib/types';

	let {
		group,
		from = '',
		to = '',
		amount = 0,
		onsaved,
		oncancel
	}: {
		group: GroupDetail;
		from?: string;
		to?: string;
		amount?: number;
		onsaved: () => void;
		oncancel: () => void;
	} = $props();

	const active = $derived(group.members.filter((m) => !m.left_at));

	// svelte-ignore state_referenced_locally
	let fromMember = $state(from || group.my_member_id);
	// svelte-ignore state_referenced_locally
	let toMember = $state(to);
	// svelte-ignore state_referenced_locally
	let amountText = $state(amount ? toInputValue(amount, group.currency) : '');
	let settledOn = $state(today());
	let note = $state('');
	let busy = $state(false);
	let error = $state('');
	let fields = $state<Record<string, string>>({});

	const label = (id: string) =>
		id === group.my_member_id ? 'You' : (active.find((m) => m.id === id)?.display_name ?? '');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = '';
		fields = {};
		const cents = parseAmount(amountText, group.currency);
		if (!cents) {
			fields = { amount_cents: 'Enter an amount like 12.50' };
			return;
		}
		busy = true;
		try {
			await settlements.create(group.id, {
				from_member: fromMember,
				to_member: toMember,
				amount_cents: cents,
				settled_on: settledOn,
				note: note.trim() || null
			});
			onsaved();
		} catch (err) {
			error = errorMessage(err);
			fields = fieldErrors(err);
		} finally {
			busy = false;
		}
	}
</script>

<form class="card settle" onsubmit={submit} novalidate>
	<h3>Record a payment</h3>
	<div class="grid">
		<div>
			<label for="from">From</label>
			<select id="from" bind:value={fromMember} aria-invalid={!!fields.from_member}>
				{#each active as m (m.id)}
					<option value={m.id}>{label(m.id)}</option>
				{/each}
			</select>
			{#if fields.from_member}<p class="field-error">{fields.from_member}</p>{/if}
		</div>
		<div>
			<label for="to">To</label>
			<select id="to" bind:value={toMember} aria-invalid={!!fields.to_member}>
				<option value="" disabled>Choose…</option>
				{#each active as m (m.id)}
					<option value={m.id}>{label(m.id)}</option>
				{/each}
			</select>
			{#if fields.to_member}<p class="field-error">{fields.to_member}</p>{/if}
		</div>
		<div>
			<label for="settle-amount">Amount ({group.currency})</label>
			<input
				id="settle-amount"
				inputmode="decimal"
				bind:value={amountText}
				aria-invalid={!!fields.amount_cents}
			/>
			{#if fields.amount_cents}<p class="field-error">{fields.amount_cents}</p>{/if}
		</div>
		<div>
			<label for="settled-on">Date</label>
			<input id="settled-on" type="date" bind:value={settledOn} max={today()} />
			{#if fields.settled_on}<p class="field-error">{fields.settled_on}</p>{/if}
		</div>
		<div class="wide">
			<label for="note">Note <span class="muted">(optional)</span></label>
			<input id="note" bind:value={note} maxlength="200" placeholder="Bank transfer, cash…" />
		</div>
	</div>
	{#if error}<p class="alert" role="alert">{error}</p>{/if}
	<div class="row">
		<button class="primary" type="submit" disabled={busy}>Save payment</button>
		<button type="button" onclick={oncancel}>Cancel</button>
	</div>
</form>

<style>
	.settle {
		margin-bottom: 1.5rem;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.9rem;
		margin-bottom: 1rem;
	}

	.wide {
		grid-column: 1 / -1;
	}

	.alert {
		margin-bottom: 1rem;
	}
</style>
