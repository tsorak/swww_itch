import { createResource, createSignal, For } from "solid-js";
import * as tapi from "@tauri-apps/api";

import { background, default as Background } from "./components/Background";
import { useView } from "./context/view";

const arr = {
  moveElement: (arr, element, targetElement, beforeOrAfter) => {
    const index = arr.findIndex((v) => v == element);
    let targetIndex = arr.findIndex((v) => v == targetElement);

    if (index === -1 || targetIndex === -1) return null;

    // Add behavior of before/after zones
    // Since Array.splice shifts elements once we "newArr.splice(index...", we need to offset targetIndex accordingly
    if (beforeOrAfter === "before") {
      // if dropped in the before zone of a rightward item, target the slot before
      targetIndex += targetIndex > index ? -1 : 0;
    } else if (beforeOrAfter === "after") {
      // if dropped in the after zone of a leftward item, target the slot after
      targetIndex += targetIndex < index ? 1 : 0;
    } else {
      return null;
    }

    if (targetIndex === index) return null;

    const newArr = [...arr];
    newArr.splice(index, 1);
    newArr.splice(targetIndex, 0, element);

    return newArr;
  },
};

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

  async function rearrange(name, [beforeOrAfter, targetName]) {
    if (
      !name ||
      !targetName ||
      name == targetName ||
      (beforeOrAfter !== "before" && beforeOrAfter !== "after")
    )
      return;

    tapi.core
      .invoke("rearrange_background", {
        bg: name,
        beforeOrAfter,
        targetBg: targetName,
      })
      .then(({ moveIndex, toIndex }) => {
        console.log(`${moveIndex} ${toIndex}`);

        if (moveIndex === toIndex) return null;

        bgs[1]((prev) => {
          const updated = [...prev];
          updated.splice(moveIndex, 1);
          updated.splice(toIndex, 0, name);

          return updated;
        });
      })
      .catch((error) => {
        console.error(error);
      });
  }

  return (
    <main class="h-full flex flex-col">
      <h1 class="text-2xl font-bold mb-2">Rearrange Queued Backgrounds</h1>
      <div class="flex flex-wrap justify-center content-start gap-8 overflow-y-auto flex-grow">
        <For each={bgs[0]()} fallback={<NoBackgroundsFound />}>
          {(bg) => <Draggable name={bg} s={{ rearrange }} />}
        </For>
      </div>
    </main>
  );
}

function Draggable({ name, s }) {
  return (
    <div class="relative select-all rounded-md overflow-hidden cursor-pointer">
      <DropZone name={name} s={s} />
      <Background id={name} name={name} class="select-none" draggable />
    </div>
  );
}

function DropZone({ name, s }) {
  const onDragStart = (ev) => {
    ev.dataTransfer.setData("text", name);
    ev.dataTransfer.effectAllowed = "move";
    ev.dataTransfer.dropEffect = "move";
  };

  const onDragOver = (ev) => {
    ev.preventDefault();
    ev.target.style.background = "#08f5";
  };

  const onDragLeave = (ev) => {
    ev.preventDefault();
    ev.target.style.background = "";
  };

  const onDrop = (ev) => {
    ev.preventDefault();
    const targetName = name;
    const dragged = ev.dataTransfer.getData("text");
    ev.target.style.background = "";
    console.log("Dropped", dragged, "at", ev.target.id, targetName);

    const beforeOrAfter = ev.target.id;
    s.rearrange(dragged, [beforeOrAfter, targetName]);
  };

  return (
    <div class="absolute z-10 w-full h-full">
      <div class="relative w-full h-full flex">
        <div
          id="before"
          class="flex-grow"
          onDragStart={onDragStart}
          onDragOver={onDragOver}
          onDragLeave={onDragLeave}
          onDrop={onDrop}
        ></div>
        <div
          id="after"
          class="flex-grow"
          onDragStart={onDragStart}
          onDragOver={onDragOver}
          onDragLeave={onDragLeave}
          onDrop={onDrop}
        ></div>
      </div>
    </div>
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
