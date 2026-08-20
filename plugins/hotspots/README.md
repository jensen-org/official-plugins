# Hotspots

Ranks the services in your project by how often they change crossed with how much of the project
depends on them. A service that scores high is one where edits land often and reach far, which is
usually where refactoring pays off and where a regression hurts most.

## What you get

A **Hotspots** page listing every ranked service with its commit count, distinct author count,
inbound and outbound import counts, and a score bar.

The score is the product of two normalised terms:

```
score = (commits / peakCommits) x ((inbound + outbound) / peakCoupling)
```

Because it is a product rather than a sum, a service has to be *both* churn-heavy and widely depended
on to rank. A file that changes constantly but nothing imports scores near zero, and so does a
stable, heavily imported one.

Coupling counts `imports` edges only. The graph also carries `contains` edges, but those just
describe directory nesting: every service trivially contains its own files and sits inside its
parent, so counting them would give every service the same baseline and flatten the ranking.

## Using it

Open the **Hotspots** page, or run `Hotspots: rescan` from the command palette. The first scan starts
on load and the last report is cached, so reopening the page is instant and the ranking survives a
restart.

Scanning reads the last 250 commits and profiles the 15 busiest services. Those bounds are not
arbitrary: the sandbox caps a plugin at 50 syscalls per second with 4 in flight, so profiling every
service at once would trip the rate limiter before it finished.

## Permissions

| Capability | Why |
|---|---|
| `git` | Reads commit history to count changes per service and attribute authors. Never writes. |
| `graph` | Reads each service's import edges to measure how much depends on it. |

No filesystem access, no network access, no knowledge access. The plugin sends nothing anywhere; the
report is computed locally and stored in the plugin's own data slot.

## When it says nothing

- *"No services were attributed to the recent commits"* means the project graph has not been built
  yet, so commits cannot be mapped onto services. Index the project and rescan.
- *"Git history unavailable"* means the project is not a git repository, or has no commits yet.
- A service showing `n/a` for In and Out exists in git history but not in the graph, usually because
  it was deleted or lives outside the indexed roots.

## Building

```sh
npm install
npm run build     # bundles src/main.ts to main.js with esbuild
npm test          # runs the ranking unit tests
```
