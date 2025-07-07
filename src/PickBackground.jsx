import { createResource, createSignal, For } from "solid-js";
import * as tapi from "@tauri-apps/api";

import { background, default as Background } from "./components/Background";
import { useView } from "./context/view";

export default function Comp() {
  const { signals } = useView();

  const bgs = createSignal(new Array(8).fill(null));

  createResource(bgs, async ([_, set]) => {
    console.log("Waiting for routing...");
    await signals.isRouting.get();
    console.log("Loading background paths...");
    const paths = await background.list();

    // set((prev) => prev.map((last, i) => paths.at(i) ?? last));
    set(paths);
    console.log("Background paths loaded.");
  });

  return (
    <main class="h-full flex flex-col">
      <h1 class="text-2xl font-bold">Pick a background</h1>
      <div class="flex flex-wrap justify-center content-start gap-8 overflow-y-auto flex-grow">
        <For each={bgs[0]()} fallback={<NoBackgroundsFound />}>
          {(bg) => <PickBg name={bg} />}
        </For>
      </div>
    </main>
  );
}

function PickBg({ name }) {
  const onClick = () => {
    console.log(`Clicked on ${name}`);
    tapi.core.invoke("set_background", { name });
  };

  return (
    <Background
      name={name}
      class="rounded-md border-2 border-[#0000] transition hover:border-blue-500 hover:transform-[scale(1.1)_translateY(.5rem)] cursor-pointer"
      onClick={onClick}
    />
  );
}

const NoBackgroundsFound = () => {
  const [HOME] = createResource(tapi.path.homeDir, { default: "$HOME" });

  return (
    <h2>
      No backgrounds found.
      <br />
      Check that {HOME}/backgrounds/ exists and contain readable images.
      <button
        type="button"
        onClick={() => tapi.core.invoke("set_background", { name: "lul.jpg" })}
      >
        Test bg
      </button>
    </h2>
  );
};
