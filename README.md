# AgentNOC

An AI-powered network monitoring and alert analysis system that monitors network prefixes, ASNs, routing (BGP, RPKI), and other network infrastructure, providing intelligent investigation and remediation guidance for Network Operations Centers.

Still heavily a WIP.

## Running the Project

### Prerequisites
- Rust and Cargo (install from [rustup.rs](https://rustup.rs/))

### Quick Start
To run the project in development mode:

```bash
cargo run
```

To run with release optimizations:

```bash
cargo run --release
```

### Configuration
Make sure you have the necessary configuration files in place:
- `prefixes.yml` - Network prefix configuration (see `prefixes.yml.example` for reference)
- BGPAlerter should be running in the `bgpalerter/` directory

## Proposed Milestones

### Phase 1 — MVP: Incident Intelligence Agent

**Monitoring:**
- BGP (sessions, prefix reachability, hijack/leak detection, path changes, MOAS anomalies via BGPAlerter/BGPStream/RIS Live)
- RPKI (ROA changes, expiry warnings, VRP deltas, invalid announcements via Routinator/OctoRPKI/Krill)
- IRR Route Objects (route/route6 modifications, mntner/policy changes)
- SNMP Traps (link up/down, BGP session changes, high CPU/memory, environmental alarms)
- Syslog Events (interface errors, BGP/OSPF neighborship changes, hardware failures, config changes)

**Capabilities:**
- Advanced Incident Reports (multi-step investigation, root cause analysis, impact assessment, timeline reconstruction)
- Alert Triage & Deduplication (cluster related alerts, confidence scoring, reduce alert fatigue)
- On-Demand Network Queries (route validity, prefix origin, AS path lookups)
- Explainability Mode (explain route leaks, RPKI invalids, IRR filtering, AS path changes)

### Phase 2 — Reactive & Diagnostic Agent

**Monitoring:**
- PeeringDB Changes (prefix sets, AS-SET updates, location/contact changes, policy changes)
- IXP Route Server (new participants, prefix withdrawals, route storms, session drops, misconfigurations)
- IX Layer-2 Events (MAC move detection, duplicate MAC, VLAN changes, ARP floods)
- Vendor Telemetry (Juniper JTI, Cisco streaming telemetry, Arista eAPI/TerminAttr)
- DCIM/Environmental (power failures, UPS status, cooling issues, temperature anomalies)

**Capabilities:**
- Path Diagnostics (MTR/traceroute analysis, path comparison, congestion inference, AS path deviation)
- On-Demand Prefix Monitoring (track visibility, periodic probes, deviation summaries)
- Peer/Upstream Behavior Insights (route shift analysis, peer stability, path asymmetry detection)
- Change Impact Preview (predict ROA/IRR/policy change consequences before deployment)

### Phase 3 — Proactive Operational Assistant

**Monitoring:**
- Latency/Loss/Jitter Deviation (RIPE Atlas, internal probes to upstreams/IXPs/PoPs)
- MTR/Traceroute Anomalies (path deviation, MPLS swap, new intermediate hops, upstream congestion)
- Congestion on Peering/Transit Links (SNMP interface counters, asymmetry detection)
- Synthetic Monitoring (ThousandEyes, Catchpoint, Pingdom, custom probes)
- CDN/Edge Node Health (node status, load imbalance, PoP congestion)

**Capabilities:**
- Trend & Drift Detection (visibility drift, prefix stability, IRR/ROA drift, peer churn patterns)
- Operational Digests (daily/weekly routing summaries, ROA/IRR changes, peer instability reports)
- Customer & Peer Insights (route stability analysis, prefix anomalies, peer performance summaries)
- Long-Term Operational Health (continuous monitoring, trend analysis, early problem detection)

### Phase 4 — Semi-Autonomous Network Advisor

**Monitoring:**
- DNS Delegation Changes (NS/DS record changes, glue mismatches, delegation chain breaks)
- Anycast Node Reachability (per-region reachability changes, BGP path drift, node deflation)
- Customer Routing Events (prefix export changes, unauthorized announcements, ASN mismatches)
- Cloud Infrastructure (AWS CloudWatch, GCP Monitoring, Azure Monitor, VPC BGP, VPN tunnels)
- Kubernetes (node status, pod health, CNI networking errors, ingress controller failures)
- DDoS Detection (Arbor/TMS, FastNetMon, Cloudflare Magic Transit)
- Customer Systems (CRM/ticket system alerts, VIP customer status, SLA breach timers)

**Capabilities:**
- Safe Automated Actions (suggest IRR/ROA changes, generate route-map diffs, propose policy cleanups)
- Predictive Incident Prevention (identify flap-prone customers, detect recurring instability patterns)
- Multi-Incident Correlation (link symptoms across events to identify common root causes)
- Approval-Gated Workflows (all actions require operator confirmation before execution)