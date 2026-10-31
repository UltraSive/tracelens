import { DEFAULT_OPTIONS, type TraceOptions } from "./types";

/** The new-trace form, shared by the toolbar, advanced panel and keyboard shortcuts. */
class TraceForm {
	target = $state("");
	options = $state<Omit<TraceOptions, "target">>({ ...DEFAULT_OPTIONS });
	presetName = $state("Default");
	input: HTMLInputElement | null = null;

	focus() {
		this.input?.focus();
		this.input?.select();
	}

	build(target = this.target): TraceOptions {
		const o = $state.snapshot(this.options);
		return {
			...o,
			target: target.trim(),
			port: o.protocol === "icmp" ? null : o.port,
			multipath: o.protocol === "udp" ? o.multipath : "classic",
			sourceAddr: o.sourceAddr?.trim() || null,
			interface: o.interface?.trim() || null
		};
	}
}

export const form = new TraceForm();
