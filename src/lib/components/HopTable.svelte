<script lang="ts">
	import type { HopStats } from "$lib/types";
	import { fmtMs, lossClass } from "$lib/format";
	import Sparkline from "./Sparkline.svelte";
	import * as Tooltip from "$lib/components/ui/tooltip";

	let { hops }: { hops: HopStats[] } = $props();

	// Shared y-scale across rows: 95th-percentile-ish ceiling so one spike doesn't flatten everything.
	const graphMax = $derived.by(() => {
		const all = hops.flatMap((h) => h.samples.filter((s): s is number => s != null)).sort((a, b) => a - b);
		if (!all.length) return 1;
		return Math.max(1, all[Math.floor(all.length * 0.98)] * 1.1);
	});

	const cols = "grid-cols-[2.25rem_minmax(14rem,1fr)_4rem_3.5rem_4rem_4rem_4rem_4rem_4rem_4rem_190px]";
</script>

<div class="min-w-[960px] text-[13px]">
	<div
		class="sticky top-0 z-10 grid {cols} items-center gap-x-2 border-b bg-muted/60 px-3 py-1.5 text-[11px] font-medium tracking-wide text-muted-foreground uppercase backdrop-blur"
	>
		<div class="text-right">#</div>
		<div>Host</div>
		<div class="text-right">Loss</div>
		<div class="text-right">Snt</div>
		<div class="text-right">Last</div>
		<div class="text-right">Avg</div>
		<div class="text-right">Best</div>
		<div class="text-right">Wrst</div>
		<div class="text-right">StDev</div>
		<div class="text-right">Jttr</div>
		<div class="pl-1">Latency (ms) · {Math.round(graphMax)} max</div>
	</div>

	{#each hops as hop (hop.ttl)}
		{@const primary = hop.hosts[0]}
		<div
			class="grid {cols} items-center gap-x-2 border-b border-border/50 px-3 py-1 font-mono tabular-nums hover:bg-muted/40"
			class:font-semibold={hop.isTarget}
		>
			<div class="text-right text-muted-foreground">{hop.ttl}</div>
			<div class="selectable min-w-0 truncate font-sans">
				{#if !primary}
					<span class="text-muted-foreground">???</span>
				{:else}
					<span>{primary.hostname ?? primary.ip}</span>
					{#if primary.hostname}
						<span class="ml-1.5 text-xs text-muted-foreground">{primary.ip}</span>
					{/if}
					{#if hop.hosts.length > 1}
						<Tooltip.Root>
							<Tooltip.Trigger
								class="ml-1.5 rounded bg-secondary px-1 text-[11px] text-secondary-foreground"
							>
								+{hop.hosts.length - 1}
							</Tooltip.Trigger>
							<Tooltip.Content side="right">
								<div class="font-mono text-xs">
									{#each hop.hosts as h (h.ip)}
										<div>{h.ip} {h.hostname ? `(${h.hostname})` : ""} · {h.count}</div>
									{/each}
								</div>
							</Tooltip.Content>
						</Tooltip.Root>
					{/if}
				{/if}
			</div>
			<div class="text-right {lossClass(hop.lossPct, hop.sent)}">{hop.lossPct.toFixed(1)}%</div>
			<div class="text-right">{hop.sent}</div>
			<div class="text-right">{fmtMs(hop.lastMs)}</div>
			<div class="text-right">{hop.recv ? fmtMs(hop.avgMs) : "–"}</div>
			<div class="text-right">{fmtMs(hop.bestMs)}</div>
			<div class="text-right">{fmtMs(hop.worstMs)}</div>
			<div class="text-right">{hop.recv > 1 ? fmtMs(hop.stddevMs) : "–"}</div>
			<div class="text-right">{fmtMs(hop.jitterMs)}</div>
			<div class="pl-1"><Sparkline samples={hop.samples} max={graphMax} /></div>
		</div>
	{:else}
		<div class="px-3 py-10 text-center font-sans text-muted-foreground">Waiting for replies…</div>
	{/each}
</div>
