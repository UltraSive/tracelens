import { DEFAULT_OPTIONS, type Preset } from "./types";

export const BUILTIN_PRESETS: Preset[] = [
	{ name: "Default", builtin: true, options: { ...DEFAULT_OPTIONS } },
	{
		name: "VoIP check",
		builtin: true,
		// small UDP packets, fast rounds, marked EF like real voice traffic
		options: { ...DEFAULT_OPTIONS, protocol: "udp", packetSize: 160, intervalMs: 20, dscp: 46 }
	},
	{
		name: "Web server (TCP 443)",
		builtin: true,
		options: { ...DEFAULT_OPTIONS, protocol: "tcp", port: 443 }
	},
	{
		name: "Large packets (1400 B)",
		builtin: true,
		options: { ...DEFAULT_OPTIONS, packetSize: 1400 }
	},
	{
		name: "Multipath (Paris, UDP)",
		builtin: true,
		options: { ...DEFAULT_OPTIONS, protocol: "udp", multipath: "paris" }
	}
];
