<script lang="ts">
	import { expenses as expensesApi, groups as groupsApi } from '$lib/client';
	import { CATEGORIES } from '$lib/categories';
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

	let search = $state('');
	let categoryFilter = $state('');
	let memberFilter = $state('');

	let debounced = $state('');
	$effect(() => {
		const value = search;
		const timer = setTimeout(() => (debounced = value), 250);
		return () => clearTimeout(timer);
	});

	async function fetchPage(offset: number) {
		loading = true;
		error = '';
		try {
			const page = await expensesApi.list(group.id, {
				q: debounced,
				category: categoryFilter,
				member: memberFilter,
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
		// Re-run whenever a filter changes.
		void [debounced, categoryFilter, memberFilter, group.id];
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
	const filtered = $derived(debounced !== '' || categoryFilter !== '' || memberFilter !== '');
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

<div class="filters">
	<input
		type="search"
		placeholder="Search expenses"
		aria-label="Search expenses"
		bind:value={search}
	/>
	<select aria-label="Category" bind:value={categoryFilter}>
		<option value="">All categories</option>
		{#each CATEGORIES as c (c.id)}
			<option value={c.id}>{c.label}</option>
		{/each}
	</select>
	<select aria-label="Person" bind:value={memberFilter}>
		<option value="">Everyone</option>
		{#each group.members as m (m.id)}
			<option value={m.id}>{m.id === group.my_member_id ? 'You' : m.display_name}</option>
		{/each}
	</select>
</div>

{#if error}
	<p class="alert" role="alert">{error}</p>
{:else if !loading && items.length === 0}
	<div class="card empty">
		{#if filtered}
			<p>No expenses match these filters.</p>
		{:else}
			<p>No expenses yet.</p>
			<p class="small">Add the first one to get started.</p>
		{/if}
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

	.filters {
		display: grid;
		grid-template-columns: 1fr 180px 160px;
		gap: 0.5rem;
		margin-bottom: 1rem;
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

	@media (max-width: 640px) {
		.filters {
			grid-template-columns: 1fr 1fr;
		}

		.filters input {
			grid-column: 1 / -1;
		}
	}
</style>
