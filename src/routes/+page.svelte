<script lang="ts">
	import { onMount } from "svelte";
	import Toolbar from "$lib/components/Toolbar.svelte";
	import HopTable from "$lib/components/HopTable.svelte";
	import VerdictBanner from "$lib/components/VerdictBanner.svelte";
	import BottomPanel from "$lib/components/BottomPanel.svelte";
	import StatusBar from "$lib/components/StatusBar.svelte";
	import HistoryDialog from "$lib/components/HistoryDialog.svelte";
	import CompareDialog from "$lib/components/CompareDialog.svelte";
	import ShortcutsDialog from "$lib/components/ShortcutsDialog.svelte";
	import { Kbd } from "$lib/components/ui/kbd";
	import { store } from "$lib/traces.svelte";
	import { prefs } from "$lib/prefs.svelte";
	import { form } from "$lib/form.svelte";
	import { copyExport, saveTrace } from "$lib/exporting";
	import { isMac, modKey } from "$lib/format";
	import { statusDot } from "$lib/status";
	import XIcon from "@lucide/svelte/icons/x";
	import ArchiveIcon from "@lucide/svelte/icons/archive";
	import ShieldAlertIcon from "@lucide/svelte/icons/shield-alert";

	let historyOpen = $state(false);
	let compareOpen = $state(false);
	let shortcutsOpen = $state(false);
	let compareA = $state<string | null>(null);
	let compareB = $state<string | null>(null);

	onMount(() => {
		prefs.load();
		const unlisten = store.init();
		return () => {
			unlisten.then((f) => f());
		};
	});

	const unprivileged = $derived(store.env && store.env.privileges.needsPrivileges && !store.env.privileges.privileged);

	function openCompare(a: string | null = store.activeKey, b: string | null = null) {
		compareA = a;
		compareB = b;
		compareOpen = true;
	}

	function togglePanel(p: "chart" | "paths") {
		prefs.panel = prefs.panel === p ? "none" : p;
		prefs.save();
	}

	function moveSelection(dir: 1 | -1) {
		const hops = store.active?.snap.hops ?? [];
		if (!hops.length) return;
		const i = hops.findIndex((h) => h.ttl === store.selectedTtl);
		const next = i < 0 ? (dir === 1 ? 0 : hops.length - 1) : Math.min(Math.max(i + dir, 0), hops.length - 1);
		store.selectedTtl = hops[next].ttl;
	}

	function onkeydown(e: KeyboardEvent) {
		const mod = isMac ? e.metaKey : e.ctrlKey;
		const el = e.target as HTMLElement;
		const typing = el.closest("input, textarea, select, [contenteditable]") != null;
		const dialogOpen = document.querySelector("[role=dialog], [role=menu]") != null;
		const tab = store.active;
		const k = e.key.toLowerCase();

		if (mod && !e.altKey) {
			if (k === "l") form.focus();
			else if (k === "t") {
				store.select(null);
				form.target = "";
				form.focus();
			} else if (k === "w" && tab) store.close(tab);
			else if (k === "s" && tab) saveTrace(tab);
			else if (k === "c" && e.shiftKey && tab) copyExport(tab, "markdown");
			else if (k === "h") historyOpen = true;
			else if (k === "k") openCompare();
			else if (k === "/") shortcutsOpen = true;
			else if (/^[1-9]$/.test(k)) store.selectIndex(Number(k) - 1);
			else return;
			e.preventDefault();
			return;
		}
		if (e.ctrlKey && e.key === "Tab") {
			store.cycle(e.shiftKey ? -1 : 1);
			e.preventDefault();
			return;
		}
		if (typing || dialogOpen || e.altKey || e.metaKey || e.ctrlKey) return;
		if (e.key === " " && tab) store.togglePause(tab);
		else if (k === "r" && tab) store.reset(tab);
		else if (k === "g" && tab) togglePanel("chart");
		else if (k === "p" && tab) togglePanel("paths");
		else if (e.key === "ArrowDown") moveSelection(1);
		else if (e.key === "ArrowUp") moveSelection(-1);
		else if (e.key === "Escape") store.selectedTtl = null;
		else if (e.key === "?") shortcutsOpen = true;
		else return;
		e.preventDefault();
	}
</script>

<svelte:window {onkeydown} />

<div class="flex h-screen flex-col bg-background">
	<header class="border-b px-3 py-2">
		<Toolbar onHistory={() => (historyOpen = true)} onShortcuts={() => (shortcutsOpen = true)} />
	</header>

	{#if unprivileged}
		<div class="flex items-start gap-2 border-b bg-amber-500/10 px-3 py-1.5 text-xs text-amber-800 dark:text-amber-300">
			<ShieldAlertIcon class="mt-px size-3.5 shrink-0" />
			<span class="selectable">Running without raw-socket privileges. {store.env?.hint}</span>
		</div>
	{/if}

	{#if store.error}
		<div class="selectable flex items-start justify-between gap-2 border-b bg-destructive/10 px-3 py-1.5 text-xs whitespace-pre-line text-destructive">
			<span>{store.error}</span>
			<button class="shrink-0" onclick={() => (store.error = null)} aria-label="Dismiss"><XIcon class="size-3.5" /></button>
		</div>
	{/if}

	{#if store.tabs.length}
		<div class="flex gap-px overflow-x-auto border-b bg-muted/40 px-1 pt-1" role="tablist">
			{#each store.tabs as t, i (t.key)}
				<div
					role="tab"
					tabindex="0"
					aria-selected={t.key === store.activeKey}
					title="{t.snap.target}{i < 9 ? ` (${modKey}+${i + 1})` : ''}"
					class="group flex max-w-56 items-center gap-2 rounded-t-md border border-b-0 px-2.5 py-1 text-xs
						{t.key === store.activeKey ? 'bg-background' : 'border-transparent text-muted-foreground hover:bg-muted'}"
					onclick={() => store.select(t.key)}
					onkeydown={(e) => e.key === "Enter" && store.select(t.key)}
					onauxclick={(e) => e.button === 1 && store.close(t)}
				>
					{#if t.kind === "saved"}
						<ArchiveIcon class="size-3 shrink-0" />
					{:else}
						<span class="size-1.5 shrink-0 rounded-full {statusDot[t.snap.status]}"></span>
					{/if}
					<span class="truncate font-mono">{t.snap.target}</span>
					<button
						class="rounded p-0.5 opacity-50 hover:bg-muted-foreground/20 hover:opacity-100"
						aria-label="Close tab"
						onclick={(e) => {
							e.stopPropagation();
							store.close(t);
						}}><XIcon class="size-3" /></button
					>
				</div>
			{/each}
		</div>
	{/if}

	{#if store.active}
		{@const tab = store.active}
		{#if tab.snap.error}
			<div class="selectable border-b bg-destructive/10 p-3 text-sm whitespace-pre-line text-destructive">{tab.snap.error}</div>
		{:else}
			<VerdictBanner verdict={tab.snap.verdict} paused={tab.snap.status === "paused"} />
		{/if}
		<main class="min-h-0 flex-1 overflow-auto">
			<HopTable snap={tab.snap} />
		</main>
		{#if prefs.panel !== "none"}
			<BottomPanel snap={tab.snap} />
		{/if}
		<StatusBar {tab} />
	{:else}
		<main class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 text-sm text-muted-foreground">
			<p>Enter a host above and press <Kbd>Enter</Kbd> to start tracing.</p>
			<p class="text-xs">
				Each trace gets its own tab and keeps running in the background ·
				<button class="underline-offset-2 hover:underline" onclick={() => (historyOpen = true)}>Saved traces</button> ·
				<button class="underline-offset-2 hover:underline" onclick={() => (shortcutsOpen = true)}>Shortcuts</button>
			</p>
		</main>
	{/if}
</div>

<HistoryDialog bind:open={historyOpen} onCompare={(a, b) => openCompare(a, b)} />
<CompareDialog bind:open={compareOpen} bind:a={compareA} bind:b={compareB} />
<ShortcutsDialog bind:open={shortcutsOpen} />
