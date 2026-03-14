<script lang="ts">
	import { auth } from '$lib/client';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { session } from '$lib/session.svelte';

	let name = $state(session.user?.name ?? '');
	let profileMessage = $state('');
	let profileError = $state('');

	async function saveProfile(event: SubmitEvent) {
		event.preventDefault();
		profileMessage = '';
		profileError = '';
		try {
			session.updateUser(await auth.updateProfile(name));
			profileMessage = 'Saved';
		} catch (err) {
			profileError = fieldErrors(err).name ?? errorMessage(err);
		}
	}
</script>

<svelte:head><title>Settings · Shared expenses</title></svelte:head>

<h1>Settings</h1>

<form class="card" onsubmit={saveProfile} novalidate>
	<h2>Profile</h2>
	<p class="muted small">Signed in as {session.user?.email}</p>
	<div class="field">
		<label for="name">Name</label>
		<input id="name" bind:value={name} maxlength="60" aria-invalid={!!profileError} />
		{#if profileError}<p class="field-error">{profileError}</p>{/if}
		<p class="muted small">
			New groups use this name. Rename yourself inside a group on its People page.
		</p>
	</div>
	<div class="row">
		<button class="primary" type="submit">Save</button>
		{#if profileMessage}<span class="positive small">{profileMessage}</span>{/if}
	</div>
</form>

<style>
	form {
		max-width: 520px;
	}

	form + form {
		margin-top: 1rem;
	}

	.row {
		margin-top: 1rem;
	}
</style>
