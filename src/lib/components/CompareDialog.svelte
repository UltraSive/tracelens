<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog";
	import * as Select from "$lib/components/ui/select";
	import { store } from "$lib/traces.svelte";
	import { fmtDateTime, fmtMs, lossClass } from "$lib/format";
	import type { HopStats, SavedMeta, TraceSnapshot } from "$lib/types";

	let { open = $bindable(false), a = $bindable<string | null>(null), b = $bindable<string | null>(null) } = $props();

	let saved = $state<SavedMeta[]>([]);
	let snapA = $state<TraceSnapshot | null>(null);
	let snapB = $state<TraceSnapshot | null>(null);

	$effect(() => {
		if (open) store.listSaved().then((s) => (saved = s));
	});

	const sources = $derived([
		...store.tabs.map((t) => ({ key: t.key, label: `${t.kind === "live" ? "Open" : "Saved"}: ${t.snap.target}`, sub: t.kind === "saved" && t.savedAt ? fmtDateTime(t.savedAt) : t.snap.status })),
		...saved.filter((m) => !store.tabs.some((t) => t.key === `S${m.file}`)).map((m) => ({ key: `S${m.file}`, label: `Saved: ${m.target}`, sub: fmtDateTime(m.savedAt) }))
	]);

	async function resolve(key: string | null): Promise<TraceSnapshot | null> {
		if (!key) return null;
		const tab = store.tabs.find((t) => t.key === key);
		if (tab) return tab.snap;
		return key.startsWith("S") ? store.loadSaved(key.slice(1)) : null;
	}

	$effect(() => {
		resolve(a).then((s) => (snapA = s));
	});
	$effect(() => {
		resolve(b).then((s) => (snapB = s));
	});

	// keep live tabs fresh
	const liveA = $derived(store.tabs.find((t) => t.key === a)?.snap ?? snapA);
	const liveB = $derived(store.tabs.find((t) => t.key === b)?.snap ?? snapB);

	const rows = $derived.by(() => {
		const ttls = new Set([...(liveA?.hops ?? []), ...(liveB?.hops ?? [])].map((h) => h.ttl));
		return [...ttls].sort((x, y) => x - y).map((ttl) => ({
			ttl,
			a: liveA?.hops.find((h) => h.ttl === ttl),
			b: liveB?.hops.find((h) => h.ttl === ttl)
		}));
	});

	const hostOf = (h?: HopStats) => (h?.hosts[0] ? (h.hosts[0].hostname ?? h.hosts[0].ip) : "???");
	const ipOf = (h?: HopStats) => h?.hosts[0]?.ip;
	const labelFor = (key: string | null) => sources.find((s) => s.key === key)?.label ?? "Choose a trace…";
</script>

{#snippet picker(value: string | null, set: (v: string) => void)}
	<Select.Root type="single" value={value ?? undefined} onValueChange={set}>
		<Select.Trigger size="sm" class="w-full">{labelFor(value)}</Select.Trigger>
		<Select.Content>
			{#each sources as s (s.key)}
				<Select.Item value={s.key} label={s.label}><span class="flex-1">{s.label}</span><span class="text-xs text-muted-foreground">{s.sub}</span></Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
{/snippet}

{#snippet verdict(s: TraceSnapshot | null)}
	{#if s}<p class="mt-1 text-xs text-muted-foreground">{s.verdict.headline} · {s.rounds} rounds</p>{/if}
{/snippet}

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-5xl">
		<Dialog.Header>
			<Dialog.Title>Compare traces</Dialog.Title>
			<Dialog.Description>Hops are lined up by number. Highlighted hosts took a different route.</Dialog.Description>
		</Dialog.Header>
		<div class="grid grid-cols-2 gap-3">
			<div>{@render picker(a, (v) => (a = v))}{@render verdict(liveA)}</div>
			<div>{@render picker(b, (v) => (b = v))}{@render verdict(liveB)}</div>
		</div>
		<div class="selectable max-h-[55vh] overflow-auto rounded-md border text-[13px]">
			<table class="w-full">
				<thead class="sticky top-0 bg-muted text-[11px] text-muted-foreground uppercase">
					<tr>
						<th class="px-2 py-1 text-right">#</th>
						<th class="px-2 py-1 text-left">Host A</th>
						<th class="px-2 py-1 text-left">Host B</th>
						<th class="px-2 py-1 text-right">Loss A</th>
						<th class="px-2 py-1 text-right">Loss B</th>
						<th class="px-2 py-1 text-right">Avg A</th>
						<th class="px-2 py-1 text-right">Avg B</th>
						<th class="px-2 py-1 text-right">Δ Avg</th>
					</tr>
				</thead>
				<tbody class="font-mono tabular-nums">
					{#each rows as r (r.ttl)}
						{@const differs = ipOf(r.a) !== ipOf(r.b)}
						{@const delta = r.a?.recv && r.b?.recv ? r.b.avgMs - r.a.avgMs : null}
						<tr class="border-t {differs ? 'bg-amber-500/10' : ''}">
							<td class="px-2 py-1 text-right text-muted-foreground">{r.ttl}</td>
							<td class="max-w-56 truncate px-2 py-1 font-sans">{hostOf(r.a)}</td>
							<td class="max-w-56 truncate px-2 py-1 font-sans">{hostOf(r.b)}</td>
							<td class="px-2 py-1 text-right {r.a ? lossClass(r.a.lossPct, r.a.sent) : ''}">{r.a ? `${r.a.lossPct.toFixed(1)}%` : "–"}</td>
							<td class="px-2 py-1 text-right {r.b ? lossClass(r.b.lossPct, r.b.sent) : ''}">{r.b ? `${r.b.lossPct.toFixed(1)}%` : "–"}</td>
							<td class="px-2 py-1 text-right">{r.a?.recv ? fmtMs(r.a.avgMs) : "–"}</td>
							<td class="px-2 py-1 text-right">{r.b?.recv ? fmtMs(r.b.avgMs) : "–"}</td>
							<td class="px-2 py-1 text-right {delta == null ? '' : delta > 5 ? 'text-red-600 dark:text-red-400' : delta < -5 ? 'text-emerald-600 dark:text-emerald-400' : 'text-muted-foreground'}">
								{delta == null ? "–" : `${delta > 0 ? "+" : ""}${delta.toFixed(1)}`}
							</td>
						</tr>
					{:else}
						<tr><td colspan="8" class="p-6 text-center font-sans text-muted-foreground">Pick two traces above.</td></tr>
					{/each}
				</tbody>
			</table>
		</div>
	</Dialog.Content>
</Dialog.Root>
