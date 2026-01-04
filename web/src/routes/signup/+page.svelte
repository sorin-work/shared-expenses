<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { auth } from '$lib/client';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { session } from '$lib/session.svelte';

	let name = $state('');
	let email = $state('');
	let password = $state('');
	let busy = $state(false);
	let error = $state('');
	let fields = $state<Record<string, string>>({});

	const next = $derived(page.url.searchParams.get('next') ?? '/');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = '';
		fields = {};
		if (password.length < 8) {
			fields = { password: 'Must be at least 8 characters' };
			return;
		}
		busy = true;
		try {
			session.signIn(await auth.signup(email, name, password));
			await goto(next);
		} catch (err) {
			error = errorMessage(err);
			fields = fieldErrors(err);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Create an account · Shared expenses</title></svelte:head>

<div class="auth card">
	<h1>Create an account</h1>
	<p class="muted">Split bills with flatmates, partners and travel buddies.</p>
	<form onsubmit={submit} novalidate>
		<div class="field">
			<label for="name">Your name</label>
			<input id="name" autocomplete="name" bind:value={name} aria-invalid={!!fields.name} />
			{#if fields.name}<p class="field-error">{fields.name}</p>{/if}
		</div>
		<div class="field">
			<label for="email">Email</label>
			<input
				id="email"
				type="email"
				autocomplete="email"
				bind:value={email}
				aria-invalid={!!fields.email}
			/>
			{#if fields.email}<p class="field-error">{fields.email}</p>{/if}
		</div>
		<div class="field">
			<label for="password">Password</label>
			<input
				id="password"
				type="password"
				autocomplete="new-password"
				bind:value={password}
				aria-invalid={!!fields.password}
			/>
			{#if fields.password}
				<p class="field-error">{fields.password}</p>
			{:else}
				<p class="muted small">At least 8 characters.</p>
			{/if}
		</div>
		{#if error}<p class="alert" role="alert">{error}</p>{/if}
		<button class="primary" type="submit" disabled={busy}>
			{busy ? 'Creating account…' : 'Create account'}
		</button>
	</form>
	<p class="muted small">Already have an account? <a href="/login{page.url.search}">Sign in</a></p>
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
