<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog";
	import type { HopStats } from "$lib/types";
	import { ownerName } from "$lib/format";

	let { hop = $bindable() }: { hop: HopStats | null } = $props();
	const total = $derived(hop ? hop.hosts.reduce((n, h) => n + h.count, 0) : 0);
</script>

<Dialog.Root open={hop != null} onOpenChange={(o) => !o && (hop = null)}>
	<Dialog.Content class="sm:max-w-xl">
		{#if hop}
			<Dialog.Header>
				<Dialog.Title>Addresses seen at hop {hop.ttl}</Dialog.Title>
				<Dialog.Description>
					More than one address usually means traffic is split across parallel paths (ECMP load balancing).
				</Dialog.Description>
			</Dialog.Header>
			<div class="selectable max-h-80 overflow-auto rounded-md border text-sm">
				<table class="w-full">
					<thead class="bg-muted/60 text-xs text-muted-foreground">
						<tr><th class="px-2 py-1 text-left">Address</th><th class="px-2 py-1 text-left">Network</th><th class="px-2 py-1 text-right">Replies</th></tr>
					</thead>
					<tbody>
						{#each hop.hosts as h (h.ip)}
							<tr class="border-t">
								<td class="px-2 py-1">
									<div class="font-mono">{h.ip}</div>
									{#if h.hostname}<div class="text-xs text-muted-foreground">{h.hostname}</div>{/if}
								</td>
								<td class="px-2 py-1">{ownerName(hop, h) ?? "–"}{#if h.asn}<span class="text-xs text-muted-foreground"> AS{h.asn.asn}</span>{/if}</td>
								<td class="px-2 py-1 text-right font-mono tabular-nums">{h.count} <span class="text-muted-foreground">({((h.count / (total || 1)) * 100).toFixed(0)}%)</span></td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</Dialog.Content>
</Dialog.Root>
