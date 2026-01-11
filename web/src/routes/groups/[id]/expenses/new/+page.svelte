<script lang="ts">
	import { goto } from '$app/navigation';
	import { expenses } from '$lib/client';
	import ExpenseForm from '$lib/components/ExpenseForm.svelte';
	import { useGroup } from '$lib/group.svelte';
	import type { ExpenseInput } from '$lib/types';

	const ctx = useGroup();
	const group = $derived(ctx.group!);

	async function save(input: ExpenseInput) {
		await expenses.create(group.id, input);
		await goto(`/groups/${group.id}`);
	}
</script>

<h2>Add an expense</h2>
<ExpenseForm {group} submitLabel="Add expense" onsubmit={save} />
