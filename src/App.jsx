import { onMount, Show } from "solid-js";
import * as tapi from "@tauri-apps/api";
import * as fs from "@tauri-apps/plugin-fs";

import { ViewProvider, useView } from "./context/view";
import T from "./components/Transition";

import Dev from "./Dev";
import SwitchBackground from "./PickBackground";
import Queue from "./Queue";

export default function ContextWrapped() {
  return (
    <ViewProvider>
      <App />
    </ViewProvider>
  );
}

function App() {
  const { isView: v, goto, signals: s } = useView();

  onMount(() => goto("pick"));

  return (
    <div class="flex flex-col w-screen h-screen">
      <div class="nav flex gap-2 justify-center p-2 mb-16">
        <button type="button" onClick={() => goto("home")}>
          Home
        </button>
        <button type="button" onClick={() => goto("pick")}>
          Quick-switch
        </button>
        <button type="button" onClick={() => goto("queue")}>
          Queue
        </button>
      </div>
      <div class="relative flex-grow">
        <T s={s}>
          <Show when={v("home")}>
            <Abs c={<Dev goto={goto} />} />
          </Show>
          <Show when={v("pick")}>
            <Abs c={<SwitchBackground />} />
          </Show>
          <Show when={v("queue")}>
            <Abs c={<Queue />} />
          </Show>
        </T>
      </div>
    </div>
  );
}

function Abs(props) {
  return (
    <div
      class={"absolute w-full h-full" + (props.class ?? "")}
      style={props.style}
    >
      {props.children || props.c}
    </div>
  );
}
