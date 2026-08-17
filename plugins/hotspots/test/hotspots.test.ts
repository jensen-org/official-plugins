import assert from "node:assert/strict";
import { test } from "node:test";

import { type Edge, type Row, imports, rank, tally } from "../src/main.ts";

function commit(id: string, author: string, time: number, services: string[]) {
  return { id, author, summary: id, time, touched_services: services };
}

function row(service: string, commits: number, inbound: number, outbound: number): Row {
  return {
    service,
    path: service,
    commits,
    authors: 1,
    lastTouched: 0,
    inbound,
    outbound,
    symbols: 0,
    score: 0,
    inGraph: true,
  };
}

function edge(relation: string): Edge {
  return { src: "a", dst: "b", relation, origin: "extracted" };
}

test("tally counts commits, distinct authors, and the newest timestamp per service", () => {
  const churn = tally([
    commit("c1", "ada", 300, ["frontend", "pluginhost"]),
    commit("c2", "grace", 200, ["frontend"]),
    commit("c3", "ada", 100, ["frontend"]),
  ]);

  assert.equal(churn.get("frontend")?.commits, 3);
  assert.equal(churn.get("frontend")?.authors.size, 2);
  assert.equal(churn.get("frontend")?.lastTouched, 300);
  assert.equal(churn.get("pluginhost")?.commits, 1);
});

test("tally tolerates a commit the graph attributed to no service", () => {
  const churn = tally([
    commit("c1", "ada", 1, []),
    { id: "c2", author: "ada", summary: "c2", time: 2 } as never,
  ]);

  assert.equal(churn.size, 0);
});

test("coupling counts import edges and ignores structural contains edges", () => {
  assert.equal(imports([edge("imports"), edge("contains"), edge("imports")]), 2);
  assert.equal(imports([edge("contains"), edge("contains")]), 0);
  assert.equal(imports(undefined), 0);
});

test("a service that churns often but couples to nothing outranks nothing", () => {
  const [top] = rank([row("churny", 100, 0, 0), row("coupled", 1, 20, 20)]);

  // Churn alone is not a hotspot: nothing depends on it, so a change there stays local.
  assert.equal(top.service, "coupled");
});

test("high churn crossed with high coupling beats either axis alone", () => {
  const ranked = rank([
    row("churn-only", 50, 2, 1),
    row("both", 40, 15, 15),
    row("coupling-only", 3, 20, 18),
  ]);

  // Only the joint winner's position is asserted: the score is a product, so which single-axis
  // service places second is an artifact of the inputs, not a property worth pinning.
  assert.equal(ranked[0].service, "both");
  assert.ok(ranked[0].score > ranked[1].score);
});

test("scores stay finite when every service is uncoupled", () => {
  const ranked = rank([row("a", 5, 0, 0), row("b", 3, 0, 0)]);

  assert.ok(ranked.every((r) => Number.isFinite(r.score)));
  assert.equal(ranked[0].service, "a");
});
