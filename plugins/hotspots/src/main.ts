import { type Node, Plugin, ui } from "@jensen/plugin";

const COMMIT_WINDOW = 250;
const MAX_SERVICES = 15;

export interface CommitSummary {
  id: string;
  author: string;
  summary: string;
  time: number;
  touched_services: string[];
}

export interface Edge {
  src: string;
  dst: string;
  relation: string;
  origin: string;
}

interface ServiceExplain {
  service?: string;
  path?: string;
  modules?: string[];
  inbound?: Edge[];
  outbound?: Edge[];
  symbols_count?: number;
  unknown?: string;
}

export interface Row {
  service: string;
  path: string;
  commits: number;
  authors: number;
  lastTouched: number;
  inbound: number;
  outbound: number;
  symbols: number;
  score: number;
  inGraph: boolean;
}

interface Report {
  rows: Row[];
  commitsScanned: number;
  servicesFound: number;
  servicesProfiled: number;
  generatedAt: number;
}

type Result = { ok: true; report: Report } | { ok: false; error: string };

interface Churn {
  commits: number;
  authors: Set<string>;
  lastTouched: number;
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

// A service's edge list also carries `contains` edges, which only describe directory nesting: every
// service trivially contains its own files and sits inside its parent folder. Counting those would
// give every service the same structural baseline and blur the ranking, so coupling counts imports.
export function imports(edges: Edge[] | undefined): number {
  return (edges ?? []).filter((edge) => edge.relation === "imports").length;
}

export function tally(commits: CommitSummary[]): Map<string, Churn> {
  const churn = new Map<string, Churn>();
  for (const commit of commits) {
    for (const service of commit.touched_services ?? []) {
      let entry = churn.get(service);
      if (!entry) {
        entry = { commits: 0, authors: new Set(), lastTouched: 0 };
        churn.set(service, entry);
      }
      entry.commits++;
      entry.authors.add(commit.author);
      entry.lastTouched = Math.max(entry.lastTouched, commit.time);
    }
  }
  return churn;
}

export function rank(rows: Row[]): Row[] {
  const peakChurn = Math.max(...rows.map((r) => r.commits), 1);
  const peakCoupling = Math.max(...rows.map((r) => r.inbound + r.outbound), 1);
  for (const row of rows) {
    row.score = (row.commits / peakChurn) * ((row.inbound + row.outbound) / peakCoupling);
  }
  return rows.sort((a, b) => b.score - a.score || b.commits - a.commits);
}

export default class extends Plugin {
  private scanning: Promise<Result> | null = null;
  private last: Result | null = null;

  async onload(): Promise<void> {
    this.addPage({
      id: "hotspots.page",
      label: "Hotspots",
      icon: "workflow",
      render: () => this.view(),
    });
    this.addCommand({
      id: "hotspots.scan",
      name: "Hotspots: rescan",
      callback: () => void this.rescan(),
    });
    const cached = await this.loadData<Report>();
    if (cached) this.last = { ok: true, report: cached };
    void this.scanOnce().then(() => this.refresh());
  }

  private view(): Node[] {
    const result = this.last;
    if (!result) return [ui.spinner("Scanning the project…")];
    if (!result.ok) return [this.header(), ui.message(result.error, "warning")];

    const { rows, commitsScanned, servicesFound, servicesProfiled } = result.report;
    const peak = Math.max(...rows.map((r) => r.score), Number.EPSILON);
    const scope =
      servicesProfiled < servicesFound
        ? `top ${servicesProfiled} of ${servicesFound} services`
        : `${servicesFound} services`;

    return [
      this.header(),
      ui.text(
        "Services ranked by how often they change crossed with how much of the project depends on " +
          "them. A high score means edits land there often and reach far.",
        "muted",
      ),
      ui.table({
        columns: [
          { key: "service", label: "Service", align: "start" },
          { key: "commits", label: "Commits" },
          { key: "authors", label: "Authors" },
          { key: "inbound", label: "In" },
          { key: "outbound", label: "Out" },
          { key: "score", label: "Score", width: "120px" },
        ],
        rows: rows.map((r) => ({
          service: r.service,
          commits: r.commits,
          authors: r.authors,
          inbound: r.inGraph ? r.inbound : "n/a",
          outbound: r.inGraph ? r.outbound : "n/a",
          score: ui.bar(r.score / peak, r.score.toFixed(3)),
        })),
        empty: "No services ranked yet.",
      }),
      ui.text(`${scope} across the last ${commitsScanned} commits.`, "muted"),
    ];
  }

  private header(): Node {
    return ui.row(
      [
        ui.heading("Hotspots"),
        ui.button({ label: "Rescan", icon: "workflow", onClick: () => this.rescan() }),
      ],
      { align: "between" },
    );
  }

  private async rescan(): Promise<void> {
    this.last = null;
    this.refresh();
    await this.scanOnce();
    this.refresh();
  }

  // The palette command and the view's Rescan button can both land at once; sharing one in-flight
  // promise keeps a double trigger from spending the syscall budget twice.
  private scanOnce(): Promise<Result> {
    if (this.scanning) return this.scanning;
    this.scanning = this.scan()
      .then((result) => {
        this.last = result;
        return result;
      })
      .finally(() => {
        this.scanning = null;
      });
    return this.scanning;
  }

  private async scan(): Promise<Result> {
    let commits: CommitSummary[];
    try {
      const history = await this.git.history<{ commits: CommitSummary[] }>({
        limit: COMMIT_WINDOW,
      });
      commits = history.commits ?? [];
    } catch (err) {
      return { ok: false, error: `Git history unavailable: ${message(err)}` };
    }

    const churn = tally(commits);
    if (churn.size === 0) {
      return {
        ok: false,
        error:
          "No services were attributed to the recent commits. The project graph may not be built yet.",
      };
    }

    const ordered = [...churn.entries()].sort((a, b) => b[1].commits - a[1].commits);
    const rows: Row[] = [];
    // Sequential on purpose: the governor caps a plugin at 50 syscalls/s and 4 in flight, so a
    // Promise.all across every service trips rate_limited before it finishes.
    for (const [service, entry] of ordered.slice(0, MAX_SERVICES)) {
      let profile: ServiceExplain;
      try {
        profile = await this.graph.explainService<ServiceExplain>({ service });
      } catch (err) {
        return { ok: false, error: `Code graph unavailable: ${message(err)}` };
      }
      rows.push({
        service,
        path: profile.path ?? "",
        commits: entry.commits,
        authors: entry.authors.size,
        lastTouched: entry.lastTouched,
        inbound: imports(profile.inbound),
        outbound: imports(profile.outbound),
        symbols: profile.symbols_count ?? 0,
        score: 0,
        inGraph: profile.unknown === undefined,
      });
    }

    const report: Report = {
      rows: rank(rows),
      commitsScanned: commits.length,
      servicesFound: churn.size,
      servicesProfiled: rows.length,
      generatedAt: Date.now(),
    };
    await this.saveData(report);
    return { ok: true, report };
  }
}
