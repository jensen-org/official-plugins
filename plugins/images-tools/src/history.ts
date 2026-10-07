export interface Step {
  from: string;
  bytes: Uint8Array;
  to: string;
}

export class History {
  private readonly chains = new Map<string, Step[]>();

  push(step: Step): void {
    const chain = this.chains.get(step.from) ?? [];
    this.chains.delete(step.from);
    chain.push(step);
    this.chains.set(step.to, chain);
  }

  count(path: string): number {
    return this.chains.get(path)?.length ?? 0;
  }

  undo(path: string): Step | null {
    const chain = this.chains.get(path);
    const step = chain?.pop();
    if (!chain || !step) return null;
    this.chains.delete(path);
    if (chain.length > 0) this.chains.set(step.from, chain);
    return step;
  }

  revert(path: string): Step | null {
    const chain = this.chains.get(path);
    const first = chain?.[0];
    if (!chain || !first) return null;
    this.chains.delete(path);
    return { from: first.from, bytes: first.bytes, to: path };
  }
}
