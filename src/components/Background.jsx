import { createResource, onCleanup } from "solid-js";
import * as tapi from "@tauri-apps/api";
import * as fs from "@tauri-apps/plugin-fs";

import T from "./Transition";

const HOME = tapi.path.homeDir();

const blobWorker = {
  isWorking: false,
  jobs: [],
  run: function (job) {
    let res;
    const promise = new Promise((r) => {
      res = r;
    });

    this.jobs.push([job, res]);

    this.startWorkChecked();

    return promise;
  },
  start: async function () {
    this.isWorking = true;

    while (this.isWorking) {
      if (this.jobs.length === 0) {
        this.isWorking = false;
        break;
      }

      const [job, resolve] = this.jobs.shift();
      const result = await job();
      resolve(result);
    }
  },
  startWorkChecked: function () {
    if (!this.isWorking) {
      this.start();
    }
  },
};

export const background = {
  async list_from_fs() {
    let entries;
    try {
      const path = await tapi.path.join(await HOME, "backgrounds");

      entries = await fs.readDir(path);
    } catch (err) {
      console.error(err);
      entries = [];
    }

    const backgrounds = entries
      .filter((entry) => !entry.isDirectory)
      .map((entry) => entry.name);

    return backgrounds;
  },

  async list() {
    try {
      return await tapi.core.invoke("get_queue", {});
    } catch (_error) {
      console.warn("Failed to get queued backgrounds");
      return [];
    }
  },

  async blob(path) {
    // path = await tapi.path.join(await HOME, "backgrounds", path);

    const job = async () => {
      const data = await fs.readFile(path);

      const blob = new Blob([data]);

      const blobUrl = URL.createObjectURL(blob);

      return blobUrl;
    };

    const result = await blobWorker.run(job);
    return result;
  },
};

export default function Background({ name, width, ...props }) {
  width ??= 256;

  const fetcher = name
    ? () => background.blob(name)
    : () => new Promise(() => null);
  const [blob, { mutate: mut }] = createResource(fetcher);

  onCleanup(() => {
    mut((state) => {
      if (!state) return;
      URL.revokeObjectURL(state);

      console.debug("cleaned up", state);

      return state;
    });
  });

  return (
    <div
      class="relative"
      style={{
        width: `${width}px`,
        ["min-height"]: `${Math.floor(width / 1.777)}px`,
      }}
    >
      <T
        // type={{
        //   in: [{ opacity: 0 }, { opacity: 1 }],
        //   out: [{ display: "none" }, { display: "none" }],
        // }}
        type="fade"
      >
        <Show when={!blob.loading} fallback={<Skeleton />}>
          <img {...props} src={blob()} />
        </Show>
      </T>
    </div>
  );
}

function Skeleton() {
  return (
    <div class="absolute top-0 left-0 size-full bg-[#aaa2] animate-pulse rounded-md" />
  );
}
