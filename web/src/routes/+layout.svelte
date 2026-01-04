<script lang="ts">
	import '../app.css';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { session } from '$lib/session.svelte';

	let { children } = $props();

	const PUBLIC = ['/login', '/signup'];
	const isPublic = $derived(PUBLIC.includes(page.url.pathname));

	$effect(() => {
		if (!session.token && !isPublic) {
			const next = page.url.pathname + page.url.search;
			goto(`/login${next === '/' ? '' : `?next=${encodeURIComponent(next)}`}`, {
				replaceState: true
			});
		}
	});

	function signOut() {
		session.signOut();
		goto('/login');
	}
</script>

<header>
	<nav>
		<a class="brand" href="/">Shared expenses</a>
		{#if session.user}
			<span class="spacer"></span>
			<span class="who">{session.user.name}</span>
			<button class="link-button" onclick={signOut}>Sign out</button>
		{/if}
	</nav>
</header>

<main>
	{#if session.token || isPublic}
		{@render children()}
	{/if}
</main>

<style>
	header {
		background: var(--surface);
		border-bottom: 1px solid var(--border);
	}

	nav {
		max-width: 960px;
		margin: 0 auto;
		padding: 0.75rem 1rem;
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.brand {
		font-weight: 700;
		color: var(--text);
		text-decoration: none;
	}

	.spacer {
		flex: 1;
	}

	.who {
		color: var(--muted);
		text-decoration: none;
	}

	main {
		max-width: 960px;
		margin: 0 auto;
		padding: 1.5rem 1rem 3rem;
	}
</style>
