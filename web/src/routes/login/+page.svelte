<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { auth } from '$lib/client';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { safeNext } from '$lib/navigation';
	import { session } from '$lib/session.svelte';

	let email = $state('');
	let password = $state('');
	let busy = $state(false);
	let error = $state('');
	let fields = $state<Record<string, string>>({});

	const next = $derived(safeNext(page.url.searchParams.get('next')));

	$effect(() => {
		if (session.token) goto(next, { replaceState: true });
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		error = '';
		fields = {};
		try {
			session.signIn(await auth.login(email, password));
			await goto(next);
		} catch (err) {
			error = errorMessage(err);
			fields = fieldErrors(err);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Sign in · Shared expenses</title></svelte:head>

<div class="auth card">
	<h1>Sign in</h1>
	<form onsubmit={submit} novalidate>
		<div class="field">
			<label for="email">Email</label>
			<input
				id="email"
				type="email"
				autocomplete="email"
				bind:value={email}
				aria-invalid={!!fields.email}
				required
			/>
			{#if fields.email}<p class="field-error">{fields.email}</p>{/if}
		</div>
		<div class="field">
			<label for="password">Password</label>
			<input
				id="password"
				type="password"
				autocomplete="current-password"
				bind:value={password}
				aria-invalid={!!fields.password}
				required
			/>
			{#if fields.password}<p class="field-error">{fields.password}</p>{/if}
		</div>
		{#if error}<p class="alert" role="alert">{error}</p>{/if}
		<button class="primary" type="submit" disabled={busy}>
			{busy ? 'Signing in…' : 'Sign in'}
		</button>
	</form>
	<p class="muted small">
		New here? <a href="/signup{page.url.search}">Create an account</a>
	</p>
</div>

<style>
	.auth {
		max-width: 400px;
		margin: 2rem auto;
	}

	form button {
		margin-top: 1.25rem;
		width: 100%;
	}

	form .alert {
		margin-top: 1rem;
	}
</style>
