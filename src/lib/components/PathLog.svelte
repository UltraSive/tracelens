<script lang="ts">
	import type { TraceSnapshot } from "$lib/types";
	import { fmtTime } from "$lib/format";

	let { snap }: { snap: TraceSnapshot } = $props();

	const names = $derived(
		new Map(snap.hops.flatMap((h) => h.hosts.map((x) => [x.ip, x.hostname ?? (x.asn ? x.asn.name : null)] as const)))
	);
	const events = $derived([...snap.pathEvents].reverse());
</script>

{#if events.length}
	<div class="selectable text-xs">
		<div class="sticky top-0 grid grid-cols-[6rem_3rem_1fr] gap-2 border-b bg-muted/80 px-3 py-1 font-medium text-muted-foreground uppercase backdrop-blur">
			<span>Time</span><span class="text-right">Hop</span><span>Change</span>
		</div>
		{#each events as e, i (i)}
			<div class="grid grid-cols-[6rem_3rem_1fr] gap-2 border-b border-border/50 px-3 py-1">
				<span class="font-mono text-muted-foreground tabular-nums">{fmtTime(e.at)}</span>
				<span class="text-right font-mono">{e.ttl}</span>
				<span class="truncate">
					<span class="font-mono">{e.from}</span>{#if names.get(e.from)}<span class="text-muted-foreground"> ({names.get(e.from)})</span>{/if}
					<span class="mx-1 text-muted-foreground">→</span>
					<span class="font-mono">{e.to}</span>{#if names.get(e.to)}<span class="text-muted-foreground"> ({names.get(e.to)})</span>{/if}
				</span>
			</div>
		{/each}
	</div>
{:else}
	<div class="flex h-full items-center justify-center px-6 text-center text-xs text-muted-foreground">
		No route changes yet. When a hop's address changes and stays changed for two rounds, it's logged here, which helps spot route flapping.
	</div>
{/if}
