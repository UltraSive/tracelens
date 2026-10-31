export type Protocol = "icmp" | "udp" | "tcp";
export type AddrFamily = "any" | "v4" | "v6";
export type Multipath = "classic" | "paris" | "dublin";
export type TraceStatus = "running" | "paused" | "finished" | "stopped" | "failed";
export type VerdictLevel = "pending" | "ok" | "info" | "warn" | "bad";

export interface TraceOptions {
	target: string;
	protocol: Protocol;
	family: AddrFamily;
	intervalMs: number;
	firstTtl: number;
	maxTtl: number;
	packetSize: number;
	maxRounds: number | null;
	port: number | null;
	sourceAddr: string | null;
	interface: string | null;
	dscp: number;
	multipath: Multipath;
	resolveHostnames: boolean;
}

export interface AsnInfo {
	asn: string;
	/** organisation, e.g. "Cloudflare, Inc." */
	name: string;
	/** registry handle, e.g. "CLOUDFLARENET" */
	handle: string;
	prefix: string;
	country: string;
}

export interface HostEntry {
	ip: string;
	hostname: string | null;
	asn: AsnInfo | null;
	private: boolean;
	count: number;
}

export interface HopStats {
	ttl: number;
	hosts: HostEntry[];
	sent: number;
	recv: number;
	lossPct: number;
	fwdLossPct: number;
	bwdLossPct: number;
	lastMs: number | null;
	bestMs: number | null;
	worstMs: number | null;
	avgMs: number;
	stddevMs: number;
	jitterMs: number | null;
	javgMs: number;
	jmaxMs: number | null;
	jinta: number;
	lastReply: string | null;
	nat: boolean | null;
	mplsLabels: number[];
	dscp: number | null;
	ecn: number | null;
	/** oldest first, null = lost */
	samples: (number | null)[];
	isTarget: boolean;
}

export interface PathEvent {
	at: number;
	ttl: number;
	from: string;
	to: string;
}

export interface Verdict {
	level: VerdictLevel;
	headline: string;
	details: string[];
	focusTtl: number | null;
}

export interface TraceSnapshot {
	id: number;
	target: string;
	targetAddr: string;
	options: TraceOptions;
	status: TraceStatus;
	startedAt: number;
	rounds: number;
	hops: HopStats[];
	pathEvents: PathEvent[];
	verdict: Verdict;
	error: string | null;
}

export interface SavedMeta {
	file: string;
	target: string;
	targetAddr: string;
	protocol: Protocol;
	savedAt: number;
	startedAt: number;
	rounds: number;
	hops: number;
	headline: string;
	level: VerdictLevel;
}

export interface EnvInfo {
	privileges: { privileged: boolean; needsPrivileges: boolean };
	hint: string;
	os: string;
	version: string;
}

export type ExportFormat = "text" | "markdown" | "csv" | "json";

export interface Preset {
	name: string;
	builtin?: boolean;
	options: Omit<TraceOptions, "target">;
}

export const DEFAULT_OPTIONS: Omit<TraceOptions, "target"> = {
	protocol: "icmp",
	family: "any",
	intervalMs: 1000,
	firstTtl: 1,
	maxTtl: 30,
	packetSize: 84,
	maxRounds: null,
	port: null,
	sourceAddr: null,
	interface: null,
	dscp: 0,
	multipath: "classic",
	resolveHostnames: true
};
