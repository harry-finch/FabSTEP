/** Reveal a contextual section on a new nonempty selection, never on a rerender. */
export class ContextReveal {
  private previous: string | null | undefined;

  next(key: string | null | undefined): boolean {
    const reveal = !!key && key !== this.previous;
    this.previous = key;
    return reveal;
  }
}
