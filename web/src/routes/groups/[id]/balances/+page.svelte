<script lang="ts">
	import { groups, settlements as settlementsApi } from '$lib/client';
	import Avatar from '$lib/components/Avatar.svelte';
	import Money from '$lib/components/Money.svelte';
	import SettlementForm from '$lib/components/SettlementForm.svelte';
	import { formatDate } from '$lib/dates';
	import { errorMessage } from '$lib/errors';
	import { useGroup } from '$lib/group.svelte';
	import type { Balances, Settlement } from '$lib/types';

	const ctx = useGroup();
	const group = $derived(ctx.group!);

	let balances = $state<Balances | null>(null);
	let history = $state<Settlement[]>([]);
	let error = $state('');
	let form = $state<{ from: string; to: string; amount: number } | null>(null);

	async function load() {
		try {
			[balances, history] = await Promise.all([
				groups.balances(group.id),
				settlementsApi.list(group.id)
			]);
		} catch (err) {
			error = errorMessage(err);
		}
	}

	$effect(() => {
		void group.id;
		load();
	});

	const who = (id: string) => (id === group.my_member_id ? 'You' : ctx.nameOf(id));

	async function saved() {
		form = null;
		await load();
	}
</script>

{#if error}<p class="alert" role="alert">{error}</p>{/if}

{#if form}
	<SettlementForm
		{group}
		from={form.from}
		to={form.to}
		amount={form.amount}
		onsaved={saved}
		oncancel={() => (form = null)}
	/>
{/if}

{#if !balances}
	<p class="muted">Loading…</p>
{:else}
	<div class="columns">
		<section class="card">
			<h2>Balances</h2>
			<ul class="balances">
				{#each balances.balances as b (b.member_id)}
					<li>
						<Avatar name={b.display_name} size={28} />
						<span class="name">
							{b.member_id === group.my_member_id ? 'You' : b.display_name}
							{#if b.left}<span class="muted small">(left)</span>{/if}
						</span>
						<Money amount={b.net} currency={group.currency} signed />
					</li>
				{/each}
			</ul>
			<p class="muted small">
				Positive means the group owes them money, negative means they owe the group.
			</p>
		</section>

		<section class="card">
			<div class="row">
				<h2>Settle up</h2>
				<span class="spacer"></span>
				<button class="small" onclick={() => (form = { from: '', to: '', amount: 0 })}>
					Record a payment
				</button>
			</div>
			{#if balances.suggestions.length === 0}
				<p class="muted">Everyone is settled up.</p>
			{:else}
				<p class="muted small">
					The fewest payments that settle everything ({balances.suggestions.length}).
				</p>
				<ul class="suggestions">
					{#each balances.suggestions as s (s.from_member + s.to_member)}
						<li>
							<span>
								<strong>{who(s.from_member)}</strong>
								{s.from_member === group.my_member_id ? 'pay' : 'pays'}
								<strong>{who(s.to_member)}</strong>
							</span>
							<span class="spacer"></span>
							<Money amount={s.amount_cents} currency={group.currency} />
							<button
								class="small"
								onclick={() =>
									(form = { from: s.from_member, to: s.to_member, amount: s.amount_cents })}
							>
								Record
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	</div>

	<section class="card payments">
		<h2>Payments</h2>
		{#if history.length === 0}
			<p class="muted">No payments recorded yet.</p>
		{:else}
			<table>
				<thead>
					<tr><th>Date</th><th>From</th><th>To</th><th class="num">Amount</th></tr>
				</thead>
				<tbody>
					{#each history as s (s.id)}
						<tr>
							<td>{formatDate(s.settled_on)}</td>
							<td>{who(s.from_member)}</td>
							<td>
								{who(s.to_member)}
								{#if s.note}<div class="muted small">{s.note}</div>{/if}
							</td>
							<td class="num"><Money amount={s.amount_cents} currency={group.currency} /></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</section>
{/if}

<style>
	.columns {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 1rem;
	}

	h2 {
		margin: 0 0 0.75rem;
	}

	ul {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.balances li,
	.suggestions li {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.45rem 0;
		border-bottom: 1px solid var(--border);
	}

	.name {
		min-width: 6rem;
	}

	.payments {
		margin-top: 1rem;
	}
</style>
