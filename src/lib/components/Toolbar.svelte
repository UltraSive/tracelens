<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import * as Select from "$lib/components/ui/select";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import PlayIcon from "@lucide/svelte/icons/play";
	import SlidersIcon from "@lucide/svelte/icons/sliders-horizontal";
	import ArchiveIcon from "@lucide/svelte/icons/archive";
	import KeyboardIcon from "@lucide/svelte/icons/keyboard";
	import TargetInput from "./TargetInput.svelte";
	import AdvancedPanel from "./AdvancedPanel.svelte";
	import { store } from "$lib/traces.svelte";
	import { prefs } from "$lib/prefs.svelte";
	import { form } from "$lib/form.svelte";
	import { modKey } from "$lib/format";

	let { onHistory, onShortcuts }: { onHistory: () => void; onShortcuts: () => void } = $props();

	const protocols = [
		{ value: "icmp", label: "ICMP" },
		{ value: "udp", label: "UDP" },
		{ value: "tcp", label: "TCP" }
	];
	const families = [
		{ value: "any", label: "IPv4/6" },
		{ value: "v4", label: "IPv4" },
		{ value: "v6", label: "IPv6" }
	];
	const intervals = [
		{ value: "20", label: "20 ms" },
		{ value: "100", label: "0.1 s" },
		{ value: "250", label: "0.25 s" },
		{ value: "500", label: "0.5 s" },
		{ value: "1000", label: "1 s" },
		{ value: "2000", label: "2 s" },
		{ value: "5000", label: "5 s" }
	];
	const labelOf = (list: { value: string; label: string }[], v: string) => list.find((o) => o.value === v)?.label ?? v;

	let interval = $derived(String(form.options.intervalMs));

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		const opts = form.build();
		if (!opts.target) return;
		if (await store.start(opts)) prefs.addRecent(opts.target);
	}
</script>

<form class="flex items-center gap-2" onsubmit={submit}>
	<TargetInput />
	<Select.Root type="single" bind:value={form.options.protocol}>
		<Select.Trigger size="sm" class="w-20" aria-label="Protocol">{labelOf(protocols, form.options.protocol)}</Select.Trigger>
		<Select.Content>
			{#each protocols as o (o.value)}<Select.Item value={o.value} label={o.label} />{/each}
		</Select.Content>
	</Select.Root>
	<Select.Root type="single" bind:value={form.options.family}>
		<Select.Trigger size="sm" class="w-24" aria-label="Address family">{labelOf(families, form.options.family)}</Select.Trigger>
		<Select.Content>
			{#each families as o (o.value)}<Select.Item value={o.value} label={o.label} />{/each}
		</Select.Content>
	</Select.Root>
	<Select.Root type="single" value={interval} onValueChange={(v) => (form.options.intervalMs = Number(v))}>
		<Select.Trigger size="sm" class="w-20" aria-label="Interval">{labelOf(intervals, interval)}</Select.Trigger>
		<Select.Content>
			{#each intervals as o (o.value)}<Select.Item value={o.value} label={o.label} />{/each}
		</Select.Content>
	</Select.Root>
	{#if form.options.protocol !== "icmp"}
		<Input
			value={form.options.port ?? ""}
			oninput={(e) => (form.options.port = Number(e.currentTarget.value) || null)}
			placeholder={form.options.protocol === "tcp" ? "Port 80" : "Port"}
			inputmode="numeric"
			class="h-8 w-20 font-mono text-[13px]"
			aria-label="Port"
		/>
	{/if}
	<Button type="submit" size="sm" disabled={store.starting || !form.target.trim()}>
		<PlayIcon />Trace
	</Button>
	<Button
		type="button"
		variant={prefs.advancedOpen ? "secondary" : "ghost"}
		size="sm"
		aria-expanded={prefs.advancedOpen}
		onclick={() => {
			prefs.advancedOpen = !prefs.advancedOpen;
			prefs.save();
		}}
	>
		<SlidersIcon />Advanced
		{#if form.presetName !== "Default"}<span class="max-w-28 truncate text-xs text-muted-foreground">· {form.presetName}</span>{/if}
	</Button>

	<div class="ml-auto flex items-center gap-0.5">
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button {...props} type="button" variant="ghost" size="icon-sm" onclick={onHistory} aria-label="Saved traces"><ArchiveIcon /></Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content>Saved traces ({modKey}+H)</Tooltip.Content>
		</Tooltip.Root>
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button {...props} type="button" variant="ghost" size="icon-sm" onclick={onShortcuts} aria-label="Keyboard shortcuts"><KeyboardIcon /></Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content>Keyboard shortcuts ({modKey}+/)</Tooltip.Content>
		</Tooltip.Root>
	</div>
</form>

{#if prefs.advancedOpen}
	<AdvancedPanel />
{/if}
