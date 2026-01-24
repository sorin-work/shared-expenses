<script lang="ts">
	import { hue, initials } from '$lib/avatar';

	let { name, size = 32 }: { name: string; size?: number } = $props();
</script>

<span
	class="avatar"
	aria-hidden="true"
	style:--hue={hue(name)}
	style:width="{size}px"
	style:height="{size}px"
	style:font-size="{size * 0.4}px">{initials(name)}</span
>

<style>
	.avatar {
		display: inline-grid;
		place-items: center;
		flex: none;
		border-radius: 50%;
		font-weight: 600;
		color: hsl(var(--hue) 55% 25%);
		background: hsl(var(--hue) 70% 85%);
	}
</style>
