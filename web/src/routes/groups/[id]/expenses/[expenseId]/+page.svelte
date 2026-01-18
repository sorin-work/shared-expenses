<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { expenses } from '$lib/client';
	import ExpenseForm from '$lib/components/ExpenseForm.svelte';
	import { timeAgo } from '$lib/dates';
	import { errorMessage } from '$lib/errors';
	import { useGroup } from '$lib/group.svelte';
	import { diffExpenses } from '$lib/history';
	import { formatMoney } from '$lib/money';
	import type { Expense, ExpenseInput, Revision } from '$lib/types';

	const ctx = useGroup();
	const group = $derived(ctx.group!);
	const expenseId = $derived(page.params.expenseId!);

	let expense = $state<Expense | null>(null);
	let revisions = $state<Revision[]>([]);
	let error = $state('');
	let deleting = $state(false);
	// Bumped after a save so the form starts from the stored values again.
	let formKey = $state(0);

	async function load() {
		error = '';
		try {
			const [e, h] = await Promise.all([
				expenses.get(group.id, expenseId),
				expenses.history(group.id, expenseId)
			]);
			expense = e;
			revisions = h;
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
		revisions = await expenses.history(group.id, expenseId);
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

	// Newest first, each with what changed compared to the version before.
	const timeline = $derived(
		revisions
			.map((r, i) => ({
				revision: r,
				changes:
					i > 0 && r.action === 'updated'
						? diffExpenses(revisions[i - 1].snapshot, r.snapshot, ctx.nameOf, group.currency)
						: []
			}))
			.reverse()
	);
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

	<section class="card history">
		<h2>History</h2>
		<ol>
			{#each timeline as { revision, changes } (revision.revision)}
				<li>
					<div>
						<strong>{revision.changed_by_name ?? 'Someone'}</strong>
						{revision.action === 'created'
							? `added this for ${formatMoney(revision.snapshot.amount_cents, group.currency)}`
							: revision.action === 'deleted'
								? 'deleted this'
								: 'edited this'}
						<span class="muted small">· {timeAgo(revision.changed_at)}</span>
					</div>
					{#if changes.length}
						<ul class="changes small">
							{#each changes as change (change.field)}
								<li>
									<span class="muted">{change.field}:</span>
									<del>{change.from}</del> → <ins>{change.to}</ins>
								</li>
							{/each}
						</ul>
					{/if}
				</li>
			{/each}
		</ol>
	</section>
{/if}

<style>
	.head h2 {
		margin: 0;
	}

	.head {
		margin-bottom: 1rem;
	}

	.history {
		margin-top: 1.5rem;
	}

	ol {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	ol > li {
		padding: 0.6rem 0;
		border-bottom: 1px solid var(--border);
	}

	ol > li:last-child {
		border-bottom: none;
	}

	.changes {
		margin: 0.35rem 0 0;
		padding-left: 1rem;
	}

	del {
		color: var(--muted);
	}

	ins {
		text-decoration: none;
		font-weight: 500;
	}
</style>
