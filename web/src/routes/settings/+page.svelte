<script lang="ts">
	import { auth } from '$lib/client';
	import { errorMessage, fieldErrors } from '$lib/errors';
	import { session } from '$lib/session.svelte';

	let name = $state(session.user?.name ?? '');
	let profileMessage = $state('');
	let profileError = $state('');

	let currentPassword = $state('');
	let newPassword = $state('');
	let passwordMessage = $state('');
	let passwordError = $state('');
	let passwordFields = $state<Record<string, string>>({});

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

	async function changePassword(event: SubmitEvent) {
		event.preventDefault();
		passwordMessage = '';
		passwordError = '';
		passwordFields = {};
		try {
			await auth.changePassword(currentPassword, newPassword);
			currentPassword = '';
			newPassword = '';
			passwordMessage = 'Password changed';
		} catch (err) {
			passwordError = errorMessage(err);
			passwordFields = fieldErrors(err);
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

<form class="card" onsubmit={changePassword} novalidate>
	<h2>Password</h2>
	<div class="field">
		<label for="current">Current password</label>
		<input
			id="current"
			type="password"
			autocomplete="current-password"
			bind:value={currentPassword}
			aria-invalid={!!passwordFields.current_password}
		/>
		{#if passwordFields.current_password}
			<p class="field-error">{passwordFields.current_password}</p>
		{/if}
	</div>
	<div class="field">
		<label for="new">New password</label>
		<input
			id="new"
			type="password"
			autocomplete="new-password"
			bind:value={newPassword}
			aria-invalid={!!passwordFields.new_password}
		/>
		{#if passwordFields.new_password}<p class="field-error">{passwordFields.new_password}</p>{/if}
	</div>
	{#if passwordError && !Object.keys(passwordFields).length}
		<p class="alert" role="alert">{passwordError}</p>
	{/if}
	<div class="row">
		<button type="submit">Change password</button>
		{#if passwordMessage}<span class="positive small">{passwordMessage}</span>{/if}
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
