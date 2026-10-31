<script lang="ts">
	import type { HopStats, HostEntry, TraceSnapshot } from "$lib/types";
	import { COLUMNS, defaultColumns } from "$lib/columns";
	import { fmtMs, lossClass, ownerName } from "$lib/format";
	import { prefs } from "$lib/prefs.svelte";
	import { store } from "$lib/traces.svelte";
	import { form } from "$lib/form.svelte";
	import Sparkline from "./Sparkline.svelte";
	import HopAddressesDialog from "./HopAddressesDialog.svelte";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import * as ContextMenu from "$lib/components/ui/context-menu";
	import { openUrl } from "@tauri-apps/plugin-opener";
	import { toast } from "svelte-sonner";

	let { snap }: { snap: TraceSnapshot } = $props();

	const visible = $derived(prefs.columns ?? defaultColumns());
	const cols = $derived(COLUMNS.filter((c) => c.fixed || visible.includes(c.id)));
	const template = $derived(cols.map((c) => c.width).join(" "));
	const minWidth = $derived(cols.reduce((n, c) => n + (c.width.endsWith("rem") ? parseFloat(c.width) * 16 : 200), 0));

	function toggleColumn(id: string, on: boolean) {
		const cur = new Set(visible);
		if (on) cur.add(id);
		else cur.delete(id);
		prefs.columns = COLUMNS.filter((c) => cur.has(c.id)).map((c) => c.id);
		prefs.save();
	}

	// Shared y-scale: 98th percentile ceiling so one spike doesn't flatten everything.
	const graphMax = $derived.by(() => {
		const all = snap.hops.flatMap((h) => h.samples.filter((s): s is number => s != null)).sort((a, b) => a - b);
		if (!all.length) return 1;
		return Math.max(1, all[Math.floor(all.length * 0.98)] * 1.1);
	});

	let addrHop = $state<HopStats | null>(null);

	async function copy(text: string, what: string) {
		await navigator.clipboard.writeText(text);
		toast.success(`Copied ${what}`);
	}

	function traceTo(host: HostEntry) {
		store.start({ ...snap.options, target: host.ip });
		form.target = host.ip;
	}

	const num = (v: number | null | undefined) => fmtMs(v);
	const pct = (v: number, h: HopStats) => (h.sent ? `${v.toFixed(1)}%` : "–");
</script>

<div class="text-[13px]" style="min-width: {minWidth}px">
	<ContextMenu.Root>
		<ContextMenu.Trigger>
			<div
				class="sticky top-0 z-10 grid items-center gap-x-2 border-b bg-muted/80 px-3 py-1.5 text-[11px] font-medium tracking-wide text-muted-foreground uppercase backdrop-blur"
				style="grid-template-columns: {template}"
			>
				{#each cols as c (c.id)}
					<div class:text-right={c.align === "right"} class="truncate" title={c.title}>
						{c.id === "graph" ? `Latency · ${Math.round(graphMax)} ms` : c.label}
					</div>
				{/each}
			</div>
		</ContextMenu.Trigger>
		<ContextMenu.Content class="w-64">
			<ContextMenu.Label>Columns</ContextMenu.Label>
			<ContextMenu.Separator />
			{#each COLUMNS.filter((c) => !c.fixed) as c (c.id)}
				<ContextMenu.CheckboxItem
					checked={visible.includes(c.id)}
					onCheckedChange={(v) => toggleColumn(c.id, v)}
					closeOnSelect={false}
				>
					<span class="flex-1">{c.label}</span>
					<span class="truncate text-xs text-muted-foreground">{c.title}</span>
				</ContextMenu.CheckboxItem>
			{/each}
			<ContextMenu.Separator />
			<ContextMenu.Item
				onSelect={() => {
					prefs.columns = null;
					prefs.save();
				}}>Reset to defaults</ContextMenu.Item
			>
		</ContextMenu.Content>
	</ContextMenu.Root>

	{#each snap.hops as hop (hop.ttl)}
		{@const primary = hop.hosts[0]}
		{@const owner = ownerName(hop)}
		{@const focus = snap.verdict.focusTtl === hop.ttl && snap.verdict.level !== "ok"}
		<ContextMenu.Root>
			<ContextMenu.Trigger>
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<div
					role="row"
					tabindex="-1"
					class="grid items-center gap-x-2 border-b border-border/50 px-3 py-1 font-mono tabular-nums
						{store.selectedTtl === hop.ttl ? 'bg-primary/10' : focus ? 'bg-amber-500/10' : 'hover:bg-muted/40'}"
					class:font-semibold={hop.isTarget && hop.recv > 0}
					style="grid-template-columns: {template}"
					onclick={() => (store.selectedTtl = store.selectedTtl === hop.ttl ? null : hop.ttl)}
					ondblclick={() => {
						store.selectedTtl = hop.ttl;
						prefs.chartMode = "hop";
						prefs.panel = "chart";
						prefs.save();
					}}
				>
					{#each cols as c (c.id)}
						<div class="min-w-0 truncate" class:text-right={c.align === "right"}>
							{#if c.id === "ttl"}
								<span class="text-muted-foreground">{hop.ttl}</span>
							{:else if c.id === "host"}
								<span class="selectable font-sans">
									{#if !primary}
										<span class="text-muted-foreground">???</span>
									{:else}
										<span>{primary.hostname ?? primary.ip}</span>
										{#if primary.hostname}<span class="ml-1.5 text-xs text-muted-foreground">{primary.ip}</span>{/if}
										{#if hop.hosts.length > 1}
											<button
												class="ml-1.5 rounded bg-secondary px-1 text-[11px] text-secondary-foreground hover:bg-secondary/70"
												title="Multiple addresses answered at this hop"
												onclick={(e) => {
													e.stopPropagation();
													addrHop = hop;
												}}>+{hop.hosts.length - 1}</button
											>
										{/if}
									{/if}
								</span>
							{:else if c.id === "owner"}
								{#if primary?.asn && !primary.private}
									<Tooltip.Root>
										<Tooltip.Trigger class="max-w-full truncate font-sans">{owner}</Tooltip.Trigger>
										<Tooltip.Content>
											AS{primary.asn.asn} {primary.asn.handle} · {primary.asn.prefix}{primary.asn.country ? ` · ${primary.asn.country}` : ""}
										</Tooltip.Content>
									</Tooltip.Root>
								{:else}
									<span class="font-sans text-muted-foreground">{owner ?? ""}</span>
								{/if}
							{:else if c.id === "loss"}
								<span class={lossClass(hop.lossPct, hop.sent)}>{pct(hop.lossPct, hop)}</span>
							{:else if c.id === "fwdLoss"}
								{pct(hop.fwdLossPct, hop)}
							{:else if c.id === "bwdLoss"}
								{pct(hop.bwdLossPct, hop)}
							{:else if c.id === "sent"}
								{hop.sent}
							{:else if c.id === "recv"}
								{hop.recv}
							{:else if c.id === "last"}
								{num(hop.lastMs)}
							{:else if c.id === "avg"}
								{hop.recv ? num(hop.avgMs) : "–"}
							{:else if c.id === "best"}
								{num(hop.bestMs)}
							{:else if c.id === "worst"}
								{num(hop.worstMs)}
							{:else if c.id === "stddev"}
								{hop.recv > 1 ? num(hop.stddevMs) : "–"}
							{:else if c.id === "jitter"}
								{num(hop.jitterMs)}
							{:else if c.id === "javg"}
								{hop.recv > 1 ? num(hop.javgMs) : "–"}
							{:else if c.id === "jmax"}
								{num(hop.jmaxMs)}
							{:else if c.id === "jinta"}
								{hop.recv > 1 ? num(hop.jinta) : "–"}
							{:else if c.id === "reply"}
								<span class="font-sans text-xs">{hop.lastReply ?? "–"}</span>
							{:else if c.id === "nat"}
								{hop.nat == null ? "–" : hop.nat ? "yes" : "no"}
							{:else if c.id === "mpls"}
								{hop.mplsLabels.length ? hop.mplsLabels.join(",") : "–"}
							{:else if c.id === "dscp"}
								{hop.dscp ?? "–"}{hop.ecn ? `/${hop.ecn}` : ""}
							{:else if c.id === "graph"}
								<Sparkline samples={hop.samples} max={graphMax} />
							{/if}
						</div>
					{/each}
				</div>
			</ContextMenu.Trigger>
			<ContextMenu.Content class="w-60">
				{#if primary}
					<ContextMenu.Label class="font-mono text-xs">Hop {hop.ttl} · {primary.ip}</ContextMenu.Label>
					<ContextMenu.Separator />
					<ContextMenu.Item onSelect={() => copy(primary.ip, "IP address")}>Copy IP address</ContextMenu.Item>
					<ContextMenu.Item disabled={!primary.hostname} onSelect={() => copy(primary.hostname ?? "", "hostname")}
						>Copy hostname</ContextMenu.Item
					>
					<ContextMenu.Separator />
					<ContextMenu.Item onSelect={() => openUrl(`https://bgp.he.net/ip/${primary.ip}`)}
						>Look up owner (WHOIS)…</ContextMenu.Item
					>
					<ContextMenu.Item onSelect={() => traceTo(primary)}>Trace to this hop</ContextMenu.Item>
					<ContextMenu.Item onSelect={() => (addrHop = hop)}>
						Show all addresses
						{#if hop.hosts.length > 1}<ContextMenu.Shortcut>{hop.hosts.length}</ContextMenu.Shortcut>{/if}
					</ContextMenu.Item>
					<ContextMenu.Separator />
					<ContextMenu.Item
						onSelect={() => {
							store.selectedTtl = hop.ttl;
							prefs.chartMode = "hop";
							prefs.panel = "chart";
							prefs.save();
						}}>Show latency chart</ContextMenu.Item
					>
				{:else}
					<ContextMenu.Label>Hop {hop.ttl} has not replied</ContextMenu.Label>
				{/if}
			</ContextMenu.Content>
		</ContextMenu.Root>
	{:else}
		<div class="px-3 py-10 text-center font-sans text-muted-foreground">Waiting for replies…</div>
	{/each}
</div>

<HopAddressesDialog bind:hop={addrHop} />
