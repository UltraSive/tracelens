<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "$lib/components/ui/button";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import Toolbar from "$lib/components/Toolbar.svelte";
	import HopTable from "$lib/components/HopTable.svelte";
	import { store } from "$lib/traces.svelte";
	import type { TraceStatus } from "$lib/types";
	import XIcon from "@lucide/svelte/icons/x";
	import SquareIcon from "@lucide/svelte/icons/square";
	import RotateCwIcon from "@lucide/svelte/icons/rotate-cw";
	import EraserIcon from "@lucide/svelte/icons/eraser";
	import CopyIcon from "@lucide/svelte/icons/copy";
	import CheckIcon from "@lucide/svelte/icons/check";
	import ShieldAlertIcon from "@lucide/svelte/icons/shield-alert";

	onMount(() => {
		const unlisten = store.init();
		return () => {
			unlisten.then((f) => f());
		};
	});

	const dot: Record<TraceStatus, string> = {
		running: "bg-emerald-500 animate-pulse",
		finished: "bg-sky-500",
		stopped: "bg-zinc-400",
		failed: "bg-red-500"
	};

	let copied = $state<string | null>(null);
	async function copy(format: "text" | "csv" | "json") {
		if (!store.active) return;
		const text = await store.exportText(store.active.id, format);
		await navigator.clipboard.writeText(text);
		copied = format;
		setTimeout(() => (copied = null), 1200);
	}

	const unprivileged = $derived(
		store.env && store.env.privileges.needsPrivileges && !store.env.privileges.privileged
	);
</script>

<div class="flex h-screen flex-col bg-background">
	<header class="border-b px-3 py-2">
		<Toolbar />
	</header>

	{#if unprivileged}
		<div class="flex items-start gap-2 border-b bg-amber-500/10 px-3 py-1.5 text-xs text-amber-800 dark:text-amber-300">
			<ShieldAlertIcon class="mt-px size-3.5 shrink-0" />
			<span class="selectable">Running without raw-socket privileges — ICMP/UDP only work if the OS allows unprivileged sockets. {store.env?.hint}</span>
		</div>
	{/if}

	{#if store.error}
		<div class="selectable flex items-start justify-between gap-2 border-b bg-destructive/10 px-3 py-1.5 text-xs whitespace-pre-line text-destructive">
			<span>{store.error}</span>
			<button class="shrink-0" onclick={() => (store.error = null)} aria-label="Dismiss"><XIcon class="size-3.5" /></button>
		</div>
	{/if}

	{#if store.traces.length}
		<div class="flex gap-px overflow-x-auto border-b bg-muted/40 px-1 pt-1" role="tablist">
			{#each store.traces as t (t.id)}
				<div
					role="tab"
					tabindex="0"
					aria-selected={t.id === store.activeId}
					class="group flex max-w-56 items-center gap-2 rounded-t-md border border-b-0 px-2.5 py-1 text-xs
						{t.id === store.activeId ? 'bg-background' : 'border-transparent text-muted-foreground hover:bg-muted'}"
					onclick={() => (store.activeId = t.id)}
					onkeydown={(e) => e.key === "Enter" && (store.activeId = t.id)}
					onauxclick={(e) => e.button === 1 && store.remove(t.id)}
				>
					<span class="size-1.5 shrink-0 rounded-full {dot[t.status]}"></span>
					<span class="truncate font-mono">{t.target}</span>
					<button
						class="rounded p-0.5 opacity-50 hover:bg-muted-foreground/20 hover:opacity-100"
						aria-label="Close trace"
						onclick={(e) => {
							e.stopPropagation();
							store.remove(t.id);
						}}><XIcon class="size-3" /></button
					>
				</div>
			{/each}
		</div>
	{/if}

	<main class="min-h-0 flex-1 overflow-auto">
		{#if store.active}
			{#if store.active.error}
				<div class="selectable m-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm whitespace-pre-line text-destructive">
					{store.active.error}
				</div>
			{/if}
			<HopTable hops={store.active.hops} />
		{:else}
			<div class="flex h-full flex-col items-center justify-center gap-1 text-sm text-muted-foreground">
				<p>Enter a host above and press <kbd class="rounded border px-1 font-mono text-xs">Enter</kbd> to start tracing.</p>
				<p class="text-xs">Each trace opens in its own tab and keeps running in the background.</p>
			</div>
		{/if}
	</main>

	{#if store.active}
		{@const t = store.active}
		<footer class="flex items-center gap-3 border-t bg-muted/40 px-3 py-1 text-xs text-muted-foreground">
			<span class="flex items-center gap-1.5"><span class="size-1.5 rounded-full {dot[t.status]}"></span>{t.status}</span>
			<span class="selectable font-mono">{t.target} → {t.targetAddr}</span>
			<span>{t.protocol.toUpperCase()}</span>
			<span>{t.rounds} rounds</span>
			<span>{t.hops.length} hops</span>

			<div class="ml-auto flex items-center gap-0.5">
				{#if t.status === "running"}
					<Button variant="ghost" size="xs" onclick={() => store.stop(t.id)}><SquareIcon />Stop</Button>
				{:else}
					<Button variant="ghost" size="xs" onclick={() => store.restart(t.id)}><RotateCwIcon />Restart</Button>
				{/if}
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="ghost" size="xs" onclick={() => store.reset(t.id)}><EraserIcon />Reset</Button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content>Clear statistics, keep tracing</Tooltip.Content>
				</Tooltip.Root>
				<span class="mx-1 h-4 w-px bg-border"></span>
				{#each [["text", "Report"], ["csv", "CSV"], ["json", "JSON"]] as const as [fmt, label] (fmt)}
					<Button variant="ghost" size="xs" onclick={() => copy(fmt)}>
						{#if copied === fmt}<CheckIcon />{:else}<CopyIcon />{/if}{label}
					</Button>
				{/each}
			</div>
		</footer>
	{/if}
</div>
