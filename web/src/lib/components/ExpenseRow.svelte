<script lang="ts">
	import { category } from '$lib/categories';
	import { formatDate } from '$lib/dates';
	import { formatMoney } from '$lib/money';
	import type { Expense } from '$lib/types';

	let {
		expense,
		currency,
		me,
		nameOf
	}: {
		expense: Expense;
		currency: string;
		me: string;
		nameOf: (id: string) => string;
	} = $props();

	const cat = $derived(category(expense.category));
	const myShare = $derived(expense.shares.find((s) => s.member_id === me)?.amount_cents ?? 0);
	const iPaid = $derived(expense.paid_by === me);
	// What this expense did to my balance.
	const effect = $derived((iPaid ? expense.amount_cents : 0) - myShare);
</script>

<a class="expense" href="/groups/{expense.group_id}/expenses/{expense.id}">
	<span class="dot" style:background={cat.colour} title={cat.label}></span>
	<span class="main">
		<span class="description">{expense.description}</span>
		<span class="muted small">
			{formatDate(expense.spent_on)} · {iPaid ? 'You' : nameOf(expense.paid_by)} paid
			{formatMoney(expense.amount_cents, currency)}
		</span>
	</span>
	<span class="effect small">
		{#if effect > 0}
			<span class="positive">you lent {formatMoney(effect, currency)}</span>
		{:else if effect < 0}
			<span class="negative">you borrowed {formatMoney(-effect, currency)}</span>
		{:else}
			<span class="muted">not involved</span>
		{/if}
	</span>
</a>

<style>
	.expense {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.7rem 0.25rem;
		border-bottom: 1px solid var(--border);
		color: inherit;
		text-decoration: none;
	}

	.expense:hover .description {
		color: var(--accent);
	}

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex: none;
	}

	.main {
		flex: 1;
		display: flex;
		flex-direction: column;
	}

	.description {
		font-weight: 500;
	}

	.effect {
		text-align: right;
		white-space: nowrap;
	}
</style>
