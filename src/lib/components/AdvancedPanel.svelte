<script lang="ts">
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import { Switch } from "$lib/components/ui/switch";
	import { Button } from "$lib/components/ui/button";
	import * as Select from "$lib/components/ui/select";
	import * as Popover from "$lib/components/ui/popover";
	import SaveIcon from "@lucide/svelte/icons/save";
	import TrashIcon from "@lucide/svelte/icons/trash-2";
	import { form } from "$lib/form.svelte";
	import { prefs } from "$lib/prefs.svelte";
	import { BUILTIN_PRESETS } from "$lib/presets";
	import type { Multipath, Preset } from "$lib/types";
	import { toast } from "svelte-sonner";

	const allPresets = $derived<Preset[]>([...BUILTIN_PRESETS, ...prefs.presets]);
	const current = $derived(allPresets.find((p) => p.name === form.presetName));
	const modified = $derived(
		current != null && JSON.stringify(current.options) !== JSON.stringify($state.snapshot(form.options))
	);

	function apply(name: string) {
		const p = allPresets.find((x) => x.name === name);
		if (!p) return;
		form.options = structuredClone(p.options);
		form.presetName = p.name;
	}

	let saveOpen = $state(false);
	let newName = $state("");
	function savePreset(e: SubmitEvent) {
		e.preventDefault();
		const name = newName.trim();
		if (!name) return;
		if (BUILTIN_PRESETS.some((p) => p.name === name)) {
			toast.error("That name is used by a built-in preset");
			return;
		}
		const preset: Preset = { name, options: $state.snapshot(form.options) };
		prefs.presets = [...prefs.presets.filter((p) => p.name !== name), preset];
		prefs.save();
		form.presetName = name;
		saveOpen = false;
		newName = "";
		toast.success(`Saved preset "${name}"`);
	}

	function deletePreset() {
		prefs.presets = prefs.presets.filter((p) => p.name !== form.presetName);
		prefs.save();
		toast(`Deleted preset "${form.presetName}"`);
		form.presetName = "Default";
	}

	const numField = (v: string, fallback: number) => {
		const n = Number(v);
		return v.trim() === "" || Number.isNaN(n) ? fallback : n;
	};

	const multipaths: { value: Multipath; label: string }[] = [
		{ value: "classic", label: "Classic" },
		{ value: "paris", label: "Paris" },
		{ value: "dublin", label: "Dublin" }
	];
</script>

<div class="mt-2 grid grid-cols-[repeat(auto-fill,minmax(9.5rem,1fr))] gap-x-3 gap-y-2 rounded-md border bg-muted/30 p-3 text-xs">
	<div class="col-span-2 flex flex-col gap-1">
		<Label class="text-xs">Preset</Label>
		<div class="flex gap-1">
			<Select.Root type="single" value={form.presetName} onValueChange={apply}>
				<Select.Trigger size="sm" class="flex-1">
					{form.presetName}{#if modified}<span class="text-muted-foreground"> (modified)</span>{/if}
				</Select.Trigger>
				<Select.Content>
					<Select.Group>
						<Select.GroupHeading>Built-in</Select.GroupHeading>
						{#each BUILTIN_PRESETS as p (p.name)}<Select.Item value={p.name} label={p.name} />{/each}
					</Select.Group>
					{#if prefs.presets.length}
						<Select.Separator />
						<Select.Group>
							<Select.GroupHeading>Yours</Select.GroupHeading>
							{#each prefs.presets as p (p.name)}<Select.Item value={p.name} label={p.name} />{/each}
						</Select.Group>
					{/if}
				</Select.Content>
			</Select.Root>
			<Popover.Root bind:open={saveOpen}>
				<Popover.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="outline" size="icon-sm" aria-label="Save as preset" title="Save as preset"><SaveIcon /></Button>
					{/snippet}
				</Popover.Trigger>
				<Popover.Content class="w-64" align="start">
					<form class="flex flex-col gap-2" onsubmit={savePreset}>
						<Label for="preset-name" class="text-xs">Save current settings as</Label>
						<Input id="preset-name" bind:value={newName} placeholder="e.g. Office VPN check" class="h-8" />
						<Button type="submit" size="sm" disabled={!newName.trim()}>Save preset</Button>
					</form>
				</Popover.Content>
			</Popover.Root>
			{#if current && !current.builtin}
				<Button variant="ghost" size="icon-sm" aria-label="Delete preset" title="Delete preset" onclick={deletePreset}><TrashIcon /></Button>
			{/if}
		</div>
	</div>

	<div class="flex flex-col gap-1">
		<Label for="adv-size" class="text-xs">Packet size (bytes)</Label>
		<Input id="adv-size" class="h-8 font-mono" inputmode="numeric" value={form.options.packetSize}
			onchange={(e) => (form.options.packetSize = numField(e.currentTarget.value, 84))} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="adv-first" class="text-xs">First hop</Label>
		<Input id="adv-first" class="h-8 font-mono" inputmode="numeric" value={form.options.firstTtl}
			onchange={(e) => (form.options.firstTtl = numField(e.currentTarget.value, 1))} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="adv-max" class="text-xs">Max hops</Label>
		<Input id="adv-max" class="h-8 font-mono" inputmode="numeric" value={form.options.maxTtl}
			onchange={(e) => (form.options.maxTtl = numField(e.currentTarget.value, 30))} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="adv-rounds" class="text-xs">Rounds (blank = until stopped)</Label>
		<Input id="adv-rounds" class="h-8 font-mono" inputmode="numeric" placeholder="∞" value={form.options.maxRounds ?? ""}
			onchange={(e) => (form.options.maxRounds = Number(e.currentTarget.value) || null)} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="adv-dscp" class="text-xs">DSCP (0–63)</Label>
		<Input id="adv-dscp" class="h-8 font-mono" inputmode="numeric" value={form.options.dscp}
			onchange={(e) => (form.options.dscp = numField(e.currentTarget.value, 0))} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="adv-src" class="text-xs">Source address</Label>
		<Input id="adv-src" class="h-8 font-mono" placeholder="auto" value={form.options.sourceAddr ?? ""}
			onchange={(e) => (form.options.sourceAddr = e.currentTarget.value.trim() || null)} />
	</div>
	<div class="flex flex-col gap-1">
		<Label for="adv-if" class="text-xs">Interface</Label>
		<Input id="adv-if" class="h-8 font-mono" placeholder="auto" value={form.options.interface ?? ""}
			onchange={(e) => (form.options.interface = e.currentTarget.value.trim() || null)} />
	</div>
	<div class="flex flex-col gap-1">
		<Label class="text-xs">Multipath (ECMP)</Label>
		<Select.Root type="single" bind:value={form.options.multipath} disabled={form.options.protocol !== "udp"}>
			<Select.Trigger size="sm" title={form.options.protocol !== "udp" ? "Paris and Dublin need UDP" : undefined}>
				{form.options.protocol === "udp" ? multipaths.find((m) => m.value === form.options.multipath)?.label : "Classic (UDP only)"}
			</Select.Trigger>
			<Select.Content>
				{#each multipaths as m (m.value)}<Select.Item value={m.value} label={m.label} />{/each}
			</Select.Content>
		</Select.Root>
	</div>
	<div class="flex items-end gap-2 pb-1.5">
		<Switch id="adv-dns" bind:checked={form.options.resolveHostnames} />
		<Label for="adv-dns" class="text-xs">Look up names &amp; networks</Label>
	</div>
</div>
