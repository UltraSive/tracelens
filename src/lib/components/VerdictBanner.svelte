<script lang="ts">
	import type { Verdict } from "$lib/types";
	import { store } from "$lib/traces.svelte";
	import CircleCheckIcon from "@lucide/svelte/icons/circle-check";
	import InfoIcon from "@lucide/svelte/icons/info";
	import TriangleAlertIcon from "@lucide/svelte/icons/triangle-alert";
	import OctagonAlertIcon from "@lucide/svelte/icons/octagon-alert";
	import LoaderIcon from "@lucide/svelte/icons/loader-circle";

	let { verdict, paused = false }: { verdict: Verdict; paused?: boolean } = $props();

	const styles = {
		pending: { cls: "border-l-zinc-400 text-muted-foreground", icon: LoaderIcon },
		ok: { cls: "border-l-emerald-500", icon: CircleCheckIcon },
		info: { cls: "border-l-sky-500", icon: InfoIcon },
		warn: { cls: "border-l-amber-500", icon: TriangleAlertIcon },
		bad: { cls: "border-l-red-500", icon: OctagonAlertIcon }
	} as const;
	const iconColor = {
		pending: "text-muted-foreground",
		ok: "text-emerald-600 dark:text-emerald-400",
		info: "text-sky-600 dark:text-sky-400",
		warn: "text-amber-600 dark:text-amber-400",
		bad: "text-red-600 dark:text-red-400"
	} as const;
	const s = $derived(styles[verdict.level]);
</script>

<section class="selectable border-b border-l-4 bg-muted/20 px-3 py-2 {s.cls}" aria-live="polite">
	<div class="flex items-center gap-2 text-sm font-medium">
		<s.icon class="size-4 shrink-0 {iconColor[verdict.level]} {verdict.level === 'pending' && !paused ? 'animate-spin' : ''}" />
		<span>{verdict.headline}</span>
		{#if verdict.focusTtl != null && verdict.level !== "ok"}
			<button
				class="ml-1 rounded px-1.5 text-xs text-muted-foreground underline-offset-2 hover:underline"
				onclick={() => (store.selectedTtl = verdict.focusTtl)}>Show hop {verdict.focusTtl}</button
			>
		{/if}
	</div>
	{#if verdict.details.length}
		<ul class="mt-1 ml-6 list-disc space-y-0.5 text-xs text-muted-foreground">
			{#each verdict.details as d, i (i)}<li>{d}</li>{/each}
		</ul>
	{/if}
</section>
