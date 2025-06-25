export default class Routing {
  isRouting = false;
  resolveHandle;
  handles = [];

  async get() {
    let res;
    const p = new Promise((r) => {
      res = r;
    });

    this.handles.push(res);

    this.check();

    return p;
  }

  check() {
    // console.debug("Checking routing status\nStatus:", this.isRouting);
    if (!this.isRouting) {
      this.handles.forEach((resolver) => {
        resolver();
      });
      this.handles = [];
    }
  }

  set(value) {
    return this.setIsRouting(value);
  }

  setIsRouting(b = true) {
    this.isRouting = b;
    this.check();
  }
}
