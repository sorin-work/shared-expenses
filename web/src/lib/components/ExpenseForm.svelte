<script lang="ts">
	import { CATEGORIES } from '$lib/categories';
	import { today } from '$lib/dates';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { formatMoney, parseAmount, toInputValue } from '$lib/money';
	import {
		evenPercentages,
		formatPercent,
		parsePercent,
		previewSplit,
		type SplitRow
	} from '$lib/split';
	import type { Expense, ExpenseInput, GroupDetail, SplitMethod } from '$lib/types';

	let {
		group,
		expense = null,
		submitLabel,
		onsubmit
	}: {
		group: GroupDetail;
		expense?: Expense | null;
		submitLabel: string;
		onsubmit: (input: ExpenseInput) => Promise<void>;
	} = $props();

	const currency = $derived(group.currency);

	// People who can appear in this expense: everyone active, plus anyone who
	// left but is already part of the expense being edited.
	const people = $derived(
		group.members.filter(
			(m) =>
				!m.left_at || expense?.paid_by === m.id || expense?.shares.some((s) => s.member_id === m.id)
		)
	);

	const METHODS: { id: SplitMethod; label: string }[] = [
		{ id: 'equal', label: 'Equally' },
		{ id: 'exact', label: 'Exact amounts' },
		{ id: 'percent', label: 'Percentages' },
		{ id: 'shares', label: 'Shares' }
	];

	// svelte-ignore state_referenced_locally
	const initial = expense;

	let description = $state(initial?.description ?? '');
	// svelte-ignore state_referenced_locally
	let amountText = $state(initial ? toInputValue(initial.amount_cents, group.currency) : '');
	let spentOn = $state(initial?.spent_on ?? today());
	// svelte-ignore state_referenced_locally
	let paidBy = $state(initial?.paid_by ?? group.my_member_id);
	let category = $state(initial?.category ?? 'general');
	let method = $state<SplitMethod>(initial?.split_method ?? 'equal');

	// Per person: included or not, and the text typed for the current method.
	// svelte-ignore state_referenced_locally
	let included = $state<Record<string, boolean>>(
		Object.fromEntries(
			people.map((m) => [
				m.id,
				initial ? initial.shares.some((s) => s.member_id === m.id) : !m.left_at
			])
		)
	);
	let values = $state<Record<string, string>>(initialValues());

	function initialValues(): Record<string, string> {
		if (!initial || initial.split_method === 'equal') return {};
		return Object.fromEntries(
			initial.shares.map((s) => [
				s.member_id,
				initial.split_method === 'exact'
					? toInputValue(s.input, group.currency)
					: initial.split_method === 'percent'
						? formatPercent(s.input)
						: String(s.input)
			])
		);
	}

	let busy = $state(false);
	let error = $state('');
	let fields = $state<Record<string, string>>({});

	const amount = $derived(parseAmount(amountText, currency));
	const selected = $derived(people.filter((m) => included[m.id]));

	function parseValue(text: string | undefined): number {
		const t = (text ?? '').trim();
		if (t === '') return NaN;
		switch (method) {
			case 'exact':
				return parseAmount(t, currency) ?? NaN;
			case 'percent':
				return parsePercent(t) ?? NaN;
			case 'shares':
				return /^\d+$/.test(t) ? Number(t) : NaN;
			default:
				return 0;
		}
	}

	const rows = $derived<SplitRow[]>(
		selected.map((m) => ({ memberId: m.id, value: parseValue(values[m.id]) }))
	);
	const preview = $derived(previewSplit(amount ?? 0, method, rows));

	function changeMethod(next: SplitMethod) {
		method = next;
		// Start from a sensible default rather than empty boxes.
		if (next === 'shares') {
			values = Object.fromEntries(selected.map((m) => [m.id, '1']));
		} else if (next === 'percent') {
			const even = evenPercentages(selected.length);
			values = Object.fromEntries(selected.map((m, i) => [m.id, formatPercent(even[i])]));
		} else if (next === 'exact' && amount) {
			const even = previewSplit(amount, 'equal', rows);
			values = even.ok
				? Object.fromEntries(
						[...even.amounts].map(([id, cents]) => [id, toInputValue(cents, currency)])
					)
				: {};
		}
	}

	function remainingText(remaining: number): string {
		if (method === 'percent') {
			const pct = formatPercent(Math.abs(remaining));
			return remaining > 0 ? `${pct}% left to assign` : `${pct}% too much`;
		}
		const money = formatMoney(Math.abs(remaining), currency);
		return remaining > 0 ? `${money} left to assign` : `${money} too much`;
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = '';
		fields = {};
		if (amount === null || amount <= 0) {
			fields = { amount_cents: 'Enter an amount like 12.50' };
			return;
		}
		if (!preview.ok) {
			fields = { splits: preview.error };
			return;
		}
		busy = true;
		try {
			await onsubmit({
				description,
				amount_cents: amount,
				paid_by: paidBy,
				split_method: method,
				category,
				spent_on: spentOn,
				notes: null,
				splits: rows.map((r) => ({
					member_id: r.memberId,
					value: method === 'equal' ? 0 : r.value
				}))
			});
		} catch (err) {
			error = errorMessage(err);
			fields = fieldErrors(err);
		} finally {
			busy = false;
		}
	}
</script>

<form class="card expense-form" onsubmit={submit} novalidate>
	<div class="grid">
		<div class="wide">
			<label for="description">Description</label>
			<input
				id="description"
				bind:value={description}
				placeholder="Groceries, rent, dinner…"
				maxlength="120"
				aria-invalid={!!fields.description}
			/>
			{#if fields.description}<p class="field-error">{fields.description}</p>{/if}
		</div>
		<div>
			<label for="amount">Amount ({currency})</label>
			<input
				id="amount"
				inputmode="decimal"
				bind:value={amountText}
				placeholder="0.00"
				aria-invalid={!!fields.amount_cents}
			/>
			{#if fields.amount_cents}<p class="field-error">{fields.amount_cents}</p>{/if}
		</div>
		<div>
			<label for="spent-on">Date</label>
			<input
				id="spent-on"
				type="date"
				bind:value={spentOn}
				max={today()}
				aria-invalid={!!fields.spent_on}
			/>
			{#if fields.spent_on}<p class="field-error">{fields.spent_on}</p>{/if}
		</div>
		<div>
			<label for="paid-by">Paid by</label>
			<select id="paid-by" bind:value={paidBy} aria-invalid={!!fields.paid_by}>
				{#each people as m (m.id)}
					<option value={m.id}>{m.display_name}</option>
				{/each}
			</select>
			{#if fields.paid_by}<p class="field-error">{fields.paid_by}</p>{/if}
		</div>
		<div>
			<label for="category">Category</label>
			<select id="category" bind:value={category}>
				{#each CATEGORIES as c (c.id)}
					<option value={c.id}>{c.label}</option>
				{/each}
			</select>
		</div>
	</div>

	<fieldset>
		<legend>Split</legend>
		<div class="methods" role="radiogroup" aria-label="Split method">
			{#each METHODS as m (m.id)}
				<button
					type="button"
					role="radio"
					aria-checked={method === m.id}
					class:active={method === m.id}
					onclick={() => changeMethod(m.id)}>{m.label}</button
				>
			{/each}
		</div>

		<ul class="people">
			{#each people as m (m.id)}
				<li class:off={!included[m.id]}>
					<label class="who">
						<input type="checkbox" bind:checked={included[m.id]} />
						{m.id === group.my_member_id ? 'You' : m.display_name}
					</label>
					{#if method !== 'equal' && included[m.id]}
						<input
							class="value"
							aria-label="{method} for {m.display_name}"
							inputmode="decimal"
							bind:value={values[m.id]}
							placeholder={method === 'exact' ? '0.00' : method === 'percent' ? '%' : '1'}
						/>
						{#if method === 'percent'}<span class="muted">%</span>{/if}
					{/if}
					<span class="share">
						{#if included[m.id] && preview.ok}
							{formatMoney(preview.amounts.get(m.id) ?? 0, currency)}
						{/if}
					</span>
				</li>
			{/each}
		</ul>

		{#if !preview.ok && preview.remaining !== undefined}
			<p class="muted small">{remainingText(preview.remaining)}</p>
		{/if}
		{#if fields.splits}<p class="field-error">{fields.splits}</p>{/if}
	</fieldset>

	{#if error}<p class="alert" role="alert">{error}</p>{/if}
	<div class="row actions">
		<button class="primary" type="submit" disabled={busy}>{busy ? 'Saving…' : submitLabel}</button>
		<a class="button" href="/groups/{group.id}">Cancel</a>
	</div>
</form>

<style>
	.grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 1rem;
	}

	.wide {
		grid-column: 1 / -1;
	}

	fieldset {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		margin: 1.25rem 0;
		padding: 0.75rem 1rem 1rem;
	}

	legend {
		font-weight: 600;
		padding: 0 0.35rem;
	}

	.methods {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		margin-bottom: 0.75rem;
	}

	.methods button.active {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
	}

	.people {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.people li {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.35rem 0;
		border-bottom: 1px solid var(--border);
	}

	.people li.off {
		color: var(--muted);
	}

	.who {
		flex: 1;
		display: flex;
		gap: 0.5rem;
		align-items: center;
		margin: 0;
		font-weight: 400;
	}

	.value {
		width: 6.5rem;
		text-align: right;
	}

	.share {
		min-width: 6rem;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}

	.actions {
		margin-top: 1rem;
	}

	.alert {
		margin-top: 1rem;
	}
</style>
