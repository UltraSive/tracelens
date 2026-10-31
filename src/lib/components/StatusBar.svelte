<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import PauseIcon from "@lucide/svelte/icons/pause";
	import PlayIcon from "@lucide/svelte/icons/play";
	import RotateCwIcon from "@lucide/svelte/icons/rotate-cw";
	import EraserIcon from "@lucide/svelte/icons/eraser";
	import SaveIcon from "@lucide/svelte/icons/save";
	import ShareIcon from "@lucide/svelte/icons/share";
	import ChartIcon from "@lucide/svelte/icons/chart-line";
	import RouteIcon from "@lucide/svelte/icons/route";
	import { store, type Tab } from "$lib/traces.svelte";
	import { prefs } from "$lib/prefs.svelte";
	import { copyExport, saveExport, saveImage, saveTrace } from "$lib/exporting";
	import { fmtDateTime, modKey } from "$lib/format";
	import { statusDot } from "$lib/status";

	let { tab }: { tab: Tab } = $props();
	const t = $derived(tab.snap);
	const live = $derived(tab.kind === "live");
	const canPause = $derived(live && (t.status === "running" || t.status === "paused"));

	function toggle(panel: "chart" | "paths") {
		prefs.panel = prefs.panel === panel ? "none" : panel;
		prefs.save();
	}
</script>

<footer class="flex items-center gap-3 border-t bg-muted/40 px-3 py-1 text-xs text-muted-foreground">
	{#if live}
		<span class="flex items-center gap-1.5"><span class="size-1.5 rounded-full {statusDot[t.status]}"></span>{t.status}</span>
	{:else}
		<span>Saved {tab.savedAt ? fmtDateTime(tab.savedAt) : ""}</span>
	{/if}
	<span class="selectable truncate font-mono">{t.target} → {t.targetAddr}</span>
	<span>{t.options.protocol.toUpperCase()}</span>
	<span>{t.rounds} rounds</span>

	<div class="ml-auto flex items-center gap-0.5">
		<Button variant={prefs.panel === "chart" ? "secondary" : "ghost"} size="xs" onclick={() => toggle("chart")} title="Latency chart (G)"><ChartIcon />Chart</Button>
		<Button variant={prefs.panel === "paths" ? "secondary" : "ghost"} size="xs" onclick={() => toggle("paths")} title="Path changes (P)">
			<RouteIcon />Path changes{#if t.pathEvents.length}<span class="rounded bg-amber-500/20 px-1 text-amber-700 dark:text-amber-300">{t.pathEvents.length}</span>{/if}
		</Button>
		<span class="mx-1 h-4 w-px bg-border"></span>
		{#if canPause}
			<Button variant="ghost" size="xs" onclick={() => store.togglePause(tab)} title="Pause / resume (Space)">
				{#if t.status === "paused"}<PlayIcon />Resume{:else}<PauseIcon />Pause{/if}
			</Button>
		{/if}
		<Button variant="ghost" size="xs" onclick={() => store.restart(tab)} title={live ? "Start over with the same settings" : "Run this trace again"}>
			<RotateCwIcon />{live ? "Restart" : "Run again"}
		</Button>
		{#if live}
			<Button variant="ghost" size="xs" onclick={() => store.reset(tab)} title="Clear statistics, keep tracing (R)"><EraserIcon />Reset</Button>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="xs" onclick={() => saveTrace(tab)}><SaveIcon />Save</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content>Save to history ({modKey}+S)</Tooltip.Content>
			</Tooltip.Root>
		{/if}
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="ghost" size="xs"><ShareIcon />Export</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="end" class="w-56">
				<DropdownMenu.Label>Copy to clipboard</DropdownMenu.Label>
				<DropdownMenu.Item onSelect={() => copyExport(tab, "markdown")}>
					Markdown <span class="text-muted-foreground">(tickets, chat)</span>
					<DropdownMenu.Shortcut>{modKey}+⇧C</DropdownMenu.Shortcut>
				</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={() => copyExport(tab, "text")}>Plain-text report</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={() => copyExport(tab, "csv")}>CSV</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={() => copyExport(tab, "json")}>JSON <span class="text-muted-foreground">(trippy format)</span></DropdownMenu.Item>
				<DropdownMenu.Separator />
				<DropdownMenu.Label>Save as file</DropdownMenu.Label>
				<DropdownMenu.Item onSelect={() => saveImage(tab)}>Image (PNG)…</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={() => saveExport(tab, "markdown")}>Markdown…</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={() => saveExport(tab, "text")}>Text report…</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={() => saveExport(tab, "csv")}>CSV…</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={() => saveExport(tab, "json")}>JSON (trippy format)…</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</div>
</footer>
