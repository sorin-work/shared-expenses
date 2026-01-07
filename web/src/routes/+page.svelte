<script lang="ts">
	import { goto } from '$app/navigation';
	import { groups as groupsApi } from '$lib/client';
	import BalanceBadge from '$lib/components/BalanceBadge.svelte';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { CURRENCIES } from '$lib/money';
	import type { GroupSummary } from '$lib/types';

	let groups = $state<GroupSummary[] | null>(null);
	let loadError = $state('');

	let creating = $state(false);
	let name = $state('');
	let currency = $state('EUR');
	let busy = $state(false);
	let error = $state('');
	let fields = $state<Record<string, string>>({});

	async function load() {
		try {
			groups = await groupsApi.list();
		} catch (err) {
			loadError = errorMessage(err);
		}
	}

	$effect(() => {
		load();
	});

	async function create(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		error = '';
		fields = {};
		try {
			const group = await groupsApi.create(name, currency);
			await goto(`/groups/${group.id}`);
		} catch (err) {
			error = errorMessage(err);
			fields = fieldErrors(err);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Your groups · Shared expenses</title></svelte:head>

<div class="row heading">
	<h1>Your groups</h1>
	<span class="spacer"></span>
	{#if !creating}
		<button class="primary" onclick={() => (creating = true)}>New group</button>
	{/if}
</div>

{#if creating}
	<form class="card create" onsubmit={create} novalidate>
		<h2>New group</h2>
		<div class="grid">
			<div>
				<label for="group-name">Name</label>
				<input
					id="group-name"
					placeholder="Flat 4B, Lisbon trip…"
					bind:value={name}
					aria-invalid={!!fields.name}
				/>
				{#if fields.name}<p class="field-error">{fields.name}</p>{/if}
			</div>
			<div>
				<label for="group-currency">Currency</label>
				<select id="group-currency" bind:value={currency}>
					{#each CURRENCIES as code (code)}
						<option value={code}>{code}</option>
					{/each}
				</select>
			</div>
		</div>
		{#if error}<p class="alert" role="alert">{error}</p>{/if}
		<div class="row">
			<button class="primary" type="submit" disabled={busy}>Create group</button>
			<button type="button" onclick={() => (creating = false)}>Cancel</button>
		</div>
	</form>
{/if}

{#if loadError}
	<p class="alert" role="alert">{loadError}</p>
{:else if groups === null}
	<p class="muted">Loading…</p>
{:else if groups.length === 0 && !creating}
	<div class="card empty">
		<p>You're not in any groups yet.</p>
		<p class="small">
			Create one for your flat, your partner or your next trip, or ask a friend for their invite
			link.
		</p>
	</div>
{:else}
	<ul class="groups">
		{#each groups as group (group.id)}
			<li>
				<a class="card group" href="/groups/{group.id}">
					<div>
						<strong>{group.name}</strong>
						<div class="muted small">
							{group.member_count}
							{group.member_count === 1 ? 'person' : 'people'} · {group.currency}
						</div>
					</div>
					<BalanceBadge amount={group.my_balance} currency={group.currency} />
				</a>
			</li>
		{/each}
	</ul>
{/if}

<style>
	.heading {
		margin-bottom: 1rem;
	}

	.heading h1 {
		margin: 0;
	}

	.create {
		margin-bottom: 1.5rem;
	}

	.create .grid {
		display: grid;
		grid-template-columns: 1fr 140px;
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.create .alert {
		margin-bottom: 1rem;
	}

	.groups {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		gap: 0.75rem;
	}

	.group {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		color: inherit;
		text-decoration: none;
	}

	.group:hover {
		border-color: var(--accent);
	}
</style>
