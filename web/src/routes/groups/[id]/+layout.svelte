<script lang="ts">
	import { page } from '$app/state';
	import { GroupState, provideGroup } from '$lib/group.svelte';

	let { children } = $props();

	const ctx = new GroupState();
	provideGroup(ctx);

	$effect(() => {
		ctx.load(page.params.id!);
	});

	const base = $derived(`/groups/${page.params.id}`);
	const tabs = $derived([
		{ href: base, label: 'Expenses' },
		{ href: `${base}/balances`, label: 'Balances' },
		{ href: `${base}/activity`, label: 'Activity' },
		{ href: `${base}/members`, label: 'People' }
	]);

	function isCurrent(href: string): boolean {
		const path = page.url.pathname;
		if (href === base) return path === base || path.startsWith(`${base}/expenses`);
		return path.startsWith(href);
	}
</script>

<svelte:head>
	{#if ctx.group}<title>{ctx.group.name} · Shared expenses</title>{/if}
</svelte:head>

{#if ctx.error}
	<div class="card empty">
		<p>{ctx.error}</p>
		<a href="/">Back to your groups</a>
	</div>
{:else if !ctx.group}
	<p class="muted">Loading…</p>
{:else}
	<div class="title">
		<a href="/" class="muted small">← All groups</a>
		<h1>{ctx.group.name}</h1>
	</div>
	<nav class="tabs" aria-label="Group sections">
		{#each tabs as tab (tab.href)}
			<a href={tab.href} aria-current={isCurrent(tab.href) ? 'page' : undefined}>{tab.label}</a>
		{/each}
	</nav>
	{@render children()}
{/if}

<style>
	.title h1 {
		margin: 0.25rem 0 1rem;
	}

	.tabs {
		display: flex;
		gap: 0.25rem;
		border-bottom: 1px solid var(--border);
		margin-bottom: 1.5rem;
		overflow-x: auto;
	}

	.tabs a {
		padding: 0.5rem 0.85rem;
		color: var(--muted);
		text-decoration: none;
		white-space: nowrap;
	}

	.tabs a[aria-current='page'] {
		color: var(--text);
		font-weight: 600;
	}
</style>
