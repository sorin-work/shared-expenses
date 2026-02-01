<script lang="ts">
	import { groups } from '$lib/client';
	import { activityLink, describeActivity } from '$lib/activity';
	import Avatar from '$lib/components/Avatar.svelte';
	import { timeAgo } from '$lib/dates';
	import { errorMessage } from '$lib/errors';
	import { useGroup } from '$lib/group.svelte';
	import type { ActivityEntry } from '$lib/types';

	const ctx = useGroup();
	const group = $derived(ctx.group!);

	let items = $state<ActivityEntry[]>([]);
	let next = $state<string | null>(null);
	let loading = $state(true);
	let error = $state('');

	async function load(before: string | null) {
		loading = true;
		try {
			const feed = await groups.activity(group.id, before);
			items = before ? [...items, ...feed.items] : feed.items;
			next = feed.next_before;
		} catch (err) {
			error = errorMessage(err);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void group.id;
		load(null);
	});
</script>

{#if error}
	<p class="alert" role="alert">{error}</p>
{:else}
	<div class="card">
		{#if !loading && items.length === 0}
			<p class="empty">Nothing has happened yet.</p>
		{/if}
		<ul class="feed">
			{#each items as entry (entry.id)}
				{@const link = activityLink(entry, group.id)}
				<li>
					<Avatar name={entry.actor_name ?? '?'} size={30} />
					<div>
						{#if link}
							<a href={link}>{describeActivity(entry, group.currency)}</a>
						{:else}
							{describeActivity(entry, group.currency)}
						{/if}
						<div class="muted small" title={new Date(entry.created_at).toLocaleString()}>
							{timeAgo(entry.created_at)}
						</div>
					</div>
				</li>
			{/each}
		</ul>
		{#if next}
			<div class="more">
				<button onclick={() => load(next)} disabled={loading}>
					{loading ? 'Loading…' : 'Show older'}
				</button>
			</div>
		{/if}
	</div>
{/if}

<style>
	.feed {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.feed li {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
		padding: 0.6rem 0;
		border-bottom: 1px solid var(--border);
	}

	.feed li:last-child {
		border-bottom: none;
	}

	.feed a {
		color: inherit;
	}

	.more {
		text-align: center;
		padding-top: 1rem;
	}
</style>
