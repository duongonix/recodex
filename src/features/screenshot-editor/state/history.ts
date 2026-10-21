export class DocumentHistory<T> {
  private past: T[] = [];
  private future: T[] = [];

  constructor(private readonly clone: (value: T) => T, private readonly limit = 100) {}

  push(current: T): void {
    this.past.push(this.clone(current));
    if (this.past.length > this.limit) this.past.shift();
    this.future = [];
  }

  undo(current: T): T | null {
    const previous = this.past.pop();
    if (!previous) return null;
    this.future.push(this.clone(current));
    return this.clone(previous);
  }

  redo(current: T): T | null {
    const next = this.future.pop();
    if (!next) return null;
    this.past.push(this.clone(current));
    return this.clone(next);
  }

  get canUndo(): boolean { return this.past.length > 0; }
  get canRedo(): boolean { return this.future.length > 0; }
}

