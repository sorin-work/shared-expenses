<script lang="ts">
	import { formatMoney } from '$lib/money';

	let { amount, currency }: { amount: number; currency: string } = $props();
</script>

{#if amount > 0}
	<span class="positive">you are owed {formatMoney(amount, currency)}</span>
{:else if amount < 0}
	<span class="negative">you owe {formatMoney(-amount, currency)}</span>
{:else}
	<span class="muted">settled up</span>
{/if}
