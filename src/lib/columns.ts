export interface ColumnDef {
	id: string;
	label: string;
	/** CSS grid track */
	width: string;
	align: "left" | "right";
	title: string;
	default: boolean;
	/** can't be hidden */
	fixed?: boolean;
}

export const COLUMNS: ColumnDef[] = [
	{ id: "ttl", label: "#", width: "2.25rem", align: "right", title: "Hop number (TTL)", default: true, fixed: true },
	{ id: "host", label: "Host", width: "minmax(13rem,1.4fr)", align: "left", title: "Responding address", default: true, fixed: true },
	{ id: "owner", label: "Network", width: "minmax(8rem,1fr)", align: "left", title: "Who operates this hop (ASN)", default: true },
	{ id: "loss", label: "Loss", width: "4rem", align: "right", title: "Packet loss %", default: true },
	{ id: "fwdLoss", label: "Fwd loss", width: "4.5rem", align: "right", title: "Loss on the way there (forward path)", default: false },
	{ id: "bwdLoss", label: "Ret loss", width: "4.5rem", align: "right", title: "Loss on the way back (return path)", default: false },
	{ id: "sent", label: "Sent", width: "3.5rem", align: "right", title: "Probes sent", default: true },
	{ id: "recv", label: "Recv", width: "3.5rem", align: "right", title: "Replies received", default: false },
	{ id: "last", label: "Last", width: "4rem", align: "right", title: "Last RTT (ms)", default: true },
	{ id: "avg", label: "Avg", width: "4rem", align: "right", title: "Average RTT (ms)", default: true },
	{ id: "best", label: "Best", width: "4rem", align: "right", title: "Best RTT (ms)", default: true },
	{ id: "worst", label: "Worst", width: "4rem", align: "right", title: "Worst RTT (ms)", default: true },
	{ id: "stddev", label: "StDev", width: "4rem", align: "right", title: "Standard deviation of RTT (ms)", default: false },
	{ id: "jitter", label: "Jitter", width: "4rem", align: "right", title: "Last jitter (ms)", default: false },
	{ id: "javg", label: "J avg", width: "4rem", align: "right", title: "Average jitter (ms)", default: false },
	{ id: "jmax", label: "J max", width: "4rem", align: "right", title: "Max jitter (ms)", default: false },
	{ id: "jinta", label: "J int", width: "4rem", align: "right", title: "Interarrival jitter", default: false },
	{ id: "reply", label: "Reply", width: "6.5rem", align: "left", title: "Last ICMP reply type", default: false },
	{ id: "nat", label: "NAT", width: "3rem", align: "left", title: "NAT detected (UDP/IPv4 only)", default: false },
	{ id: "mpls", label: "MPLS", width: "6rem", align: "left", title: "MPLS labels from ICMP extensions", default: false },
	{ id: "dscp", label: "DSCP", width: "3.5rem", align: "right", title: "DSCP / ECN in the reply's quoted header", default: false },
	{ id: "graph", label: "Latency", width: "190px", align: "left", title: "Recent RTT samples; red = lost", default: true }
];

export const defaultColumns = () => COLUMNS.filter((c) => c.default).map((c) => c.id);
