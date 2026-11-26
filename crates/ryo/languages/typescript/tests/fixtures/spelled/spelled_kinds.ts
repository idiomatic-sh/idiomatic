// Every spelled kind of the language in real source.
class Holder {
  #secret = null;
  read(): unknown {
    return this.#secret;
  }
}

export const nothing: null = null;
