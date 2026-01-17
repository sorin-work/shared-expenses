<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { expenses } from '$lib/client';
	import ExpenseForm from '$lib/components/ExpenseForm.svelte';
	import { errorMessage } from '$lib/errors';
	import { useGroup } from '$lib/group.svelte';
	import type { Expense, ExpenseInput } from '$lib/types';

	const ctx = useGroup();
	const group = $derived(ctx.group!);
	const expenseId = $derived(page.params.expenseId!);

	let expense = $state<Expense | null>(null);
	let error = $state('');
	let deleting = $state(false);
	// Bumped after a save so the form starts from the stored values again.
	let formKey = $state(0);

	async function load() {
		error = '';
		try {
			expense = await expenses.get(group.id, expenseId);
		} catch (err) {
			error = errorMessage(err);
		}
	}

	$effect(() => {
		void expenseId;
		load();
	});

	async function save(input: ExpenseInput) {
		expense = await expenses.update(group.id, expenseId, input);
		formKey += 1;
	}

	async function remove() {
		if (
			!expense ||
			!confirm(`Delete "${expense.description}"? It will disappear from the balances.`)
		) {
			return;
		}
		deleting = true;
		try {
			await expenses.remove(group.id, expenseId);
			await goto(`/groups/${group.id}`);
		} catch (err) {
			error = errorMessage(err);
			deleting = false;
		}
	}
</script>

{#if error}
	<p class="alert" role="alert">{error}</p>
	<a href="/groups/{group.id}">Back to expenses</a>
{:else if !expense}
	<p class="muted">Loading…</p>
{:else}
	<div class="row head">
		<h2>{expense.description}</h2>
		<span class="spacer"></span>
		<button class="danger" onclick={remove} disabled={deleting}>Delete</button>
	</div>

	{#key formKey}
		<ExpenseForm {group} {expense} submitLabel="Save changes" onsubmit={save} />
	{/key}
{/if}

<style>
	.head h2 {
		margin: 0;
	}

	.head {
		margin-bottom: 1rem;
	}
</style>
