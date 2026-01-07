<script lang="ts">
	import { expenses as expensesApi, groups as groupsApi } from '$lib/client';
	import BalanceBadge from '$lib/components/BalanceBadge.svelte';
	import ExpenseRow from '$lib/components/ExpenseRow.svelte';
	import { formatMonth, groupByMonth } from '$lib/dates';
	import { errorMessage } from '$lib/errors';
	import { useGroup } from '$lib/group.svelte';
	import type { Expense } from '$lib/types';

	const ctx = useGroup();
	const group = $derived(ctx.group!);

	let items = $state<Expense[]>([]);
	let loading = $state(true);
	let error = $state('');
	let myBalance = $state<number | null>(null);

	async function fetchPage(offset: number) {
		loading = true;
		error = '';
		try {
			const page = await expensesApi.list(group.id, {
				offset,
				limit: 200
			});
			items = offset === 0 ? page.items : [...items, ...page.items];
		} catch (err) {
			error = errorMessage(err);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void group.id;
		fetchPage(0);
	});

	$effect(() => {
		groupsApi
			.balances(group.id)
			.then((b) => {
				myBalance = b.balances.find((r) => r.member_id === group.my_member_id)?.net ?? 0;
			})
			.catch(() => (myBalance = null));
	});

	const months = $derived(groupByMonth(items, (e) => e.spent_on));
</script>

<div class="row summary">
	<div>
		{#if myBalance !== null}
			<strong><BalanceBadge amount={myBalance} currency={group.currency} /></strong>
		{/if}
	</div>
	<span class="spacer"></span>
	<a class="button primary" href="/groups/{group.id}/expenses/new">Add expense</a>
</div>

{#if error}
	<p class="alert" role="alert">{error}</p>
{:else if !loading && items.length === 0}
	<div class="card empty">
		<p>No expenses yet.</p>
		<p class="small">Add the first one to get started.</p>
	</div>
{:else}
	<div class="card list">
		{#each months as [month, monthItems] (month)}
			<h3 class="month">{formatMonth(month)}</h3>
			{#each monthItems as expense (expense.id)}
				<ExpenseRow
					{expense}
					currency={group.currency}
					me={group.my_member_id}
					nameOf={ctx.nameOf}
				/>
			{/each}
		{/each}
	</div>
{/if}

<style>
	.summary {
		margin-bottom: 1rem;
	}

	.summary strong {
		margin-right: 0.5rem;
	}

	.list {
		padding-top: 0.5rem;
	}

	.month {
		font-size: 0.8rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--muted);
		margin: 1rem 0 0.25rem;
	}
</style>
