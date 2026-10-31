<script lang="ts">
	import type { TraceSnapshot } from "$lib/types";
	import { prefs } from "$lib/prefs.svelte";
	import { store } from "$lib/traces.svelte";
	import LatencyChart from "./LatencyChart.svelte";
	import PathLog from "./PathLog.svelte";
	import XIcon from "@lucide/svelte/icons/x";

	let { snap }: { snap: TraceSnapshot } = $props();

	const hopForChart = $derived(
		snap.hops.find((h) => h.ttl === store.selectedTtl) ?? snap.hops.find((h) => h.isTarget) ?? snap.hops.at(-1)
	);
	const chartHops = $derived(
		prefs.chartMode === "all" ? snap.hops.filter((h) => h.recv > 0) : hopForChart ? [hopForChart] : []
	);

	let height = $state(240);
	function startResize(e: PointerEvent) {
		const startY = e.clientY;
		const startH = height;
		const move = (ev: PointerEvent) => (height = Math.min(Math.max(startH + (startY - ev.clientY), 120), window.innerHeight * 0.7));
		const up = () => {
			window.removeEventListener("pointermove", move);
			window.removeEventListener("pointerup", up);
		};
		window.addEventListener("pointermove", move);
		window.addEventListener("pointerup", up);
	}

	function setPanel(p: typeof prefs.panel) {
		prefs.panel = p;
		prefs.save();
	}
</script>

<section class="flex shrink-0 flex-col border-t" style="height: {height}px">
	<div
		class="h-1 shrink-0 cursor-row-resize hover:bg-primary/30"
		role="separator"
		aria-orientation="horizontal"
		onpointerdown={startResize}
	></div>
	<div class="flex items-center gap-1 border-b px-2 py-1 text-xs">
		<button class="rounded px-2 py-0.5 {prefs.panel === 'chart' ? 'bg-secondary font-medium' : 'text-muted-foreground hover:bg-muted'}" onclick={() => setPanel("chart")}>Latency chart</button>
		<button class="rounded px-2 py-0.5 {prefs.panel === 'paths' ? 'bg-secondary font-medium' : 'text-muted-foreground hover:bg-muted'}" onclick={() => setPanel("paths")}>
			Path changes{#if snap.pathEvents.length}<span class="ml-1 rounded bg-amber-500/20 px-1 text-amber-700 dark:text-amber-300">{snap.pathEvents.length}</span>{/if}
		</button>
		{#if prefs.panel === "chart"}
			<span class="mx-2 h-4 w-px bg-border"></span>
			<button class="rounded px-2 py-0.5 {prefs.chartMode === 'hop' ? 'bg-secondary' : 'text-muted-foreground hover:bg-muted'}" onclick={() => { prefs.chartMode = "hop"; prefs.save(); }}>
				{hopForChart ? `Hop ${hopForChart.ttl}${store.selectedTtl == null && hopForChart.isTarget ? " (destination)" : ""}` : "Selected hop"}
			</button>
			<button class="rounded px-2 py-0.5 {prefs.chartMode === 'all' ? 'bg-secondary' : 'text-muted-foreground hover:bg-muted'}" onclick={() => { prefs.chartMode = "all"; prefs.save(); }}>All hops</button>
			<span class="ml-2 text-muted-foreground">Click a row to pick a hop · drag to zoom · double-click chart to reset</span>
		{/if}
		<button class="ml-auto rounded p-1 text-muted-foreground hover:bg-muted" aria-label="Close panel" onclick={() => setPanel("none")}><XIcon class="size-3.5" /></button>
	</div>
	<div class="min-h-0 flex-1 overflow-auto">
		{#if prefs.panel === "chart"}
			<div class="h-full p-2">
				{#if chartHops.length && chartHops.some((h) => h.samples.length)}
					<LatencyChart hops={chartHops} intervalMs={snap.options.intervalMs} />
				{:else}
					<div class="flex h-full items-center justify-center text-xs text-muted-foreground">No samples yet.</div>
				{/if}
			</div>
		{:else}
			<PathLog {snap} />
		{/if}
	</div>
</section>
