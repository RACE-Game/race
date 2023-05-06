export class RandomState {
  #id: bigint;
  #size: number;

  /// FIXME
  constructor(id: bigint, size: number) {
    this.#id = id;
    this.#size = size;
  }

  get id() {
    return this.#id;
  }

  get size() {
    return this.#size;
  }
}
