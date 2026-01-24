<script lang="ts">
	import { formatMoney } from '$lib/money';

	let {
		amount,
		currency,
		signed = false
	}: { amount: number; currency: string; signed?: boolean } = $props();

	const text = $derived(
		signed && amount > 0 ? `+${formatMoney(amount, currency)}` : formatMoney(amount, currency)
	);
</script>

<span class="money" class:positive={signed && amount > 0} class:negative={signed && amount < 0}
	>{text}</span
>

<style>
	.money {
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
</style>
