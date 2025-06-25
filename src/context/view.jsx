import { createContext, useContext, createSignal } from "solid-js";

import Routing from "./view/Routing";

const ctx = createContext();

export function ViewProvider(props) {
  const s = (() => {
    const [view, setView] = createSignal("pick");
    const [busyExiting, setBusyExiting] = createSignal("");

    return {
      view: { get: view, set: setView },
      busyExiting: { get: busyExiting, set: setBusyExiting },
      isRouting: new Routing(),
    };
  })();

  const v = (p) => {
    const b = s.busyExiting.get();
    const isOtherBusy = b == "" ? p : b != p;

    return isOtherBusy == p && s.view.get() == p;
  };

  const goto = (p) =>
    s.view.set((prev) => {
      s.busyExiting.set((ex) => {
        if (ex != "") return "";
        return prev;
      });
      return p;
    });

  const state = {
    signals: s,
    isView: v,
    goto,
  };

  return <ctx.Provider value={state}>{props.children}</ctx.Provider>;
}

/**
 *
 * @returns {{signals: {[k:string]: {get: Function, set: Function}}, isView: (path: string) => boolean, goto: (path: string) => void}}
 */
export function useView() {
  return useContext(ctx);
}
