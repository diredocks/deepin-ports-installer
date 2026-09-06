// TODO: select device

import { mdiFlash, mdiFolderOpenOutline, mdiHarddisk, mdiLinkVariant, mdiLaptop } from "@mdi/js";
import type { JSX } from "solid-js";
import { Icon } from "~/components/Icon";

const ghostButton =
  "flex w-full items-center justify-center gap-2 rounded-full border border-gray-300 " +
  "bg-white px-4 py-2 text-sm text-gray-600 shadow-sm transition-colors " +
  "hover:border-gray-400 hover:text-gray-900 active:bg-gray-100";

type StepProps = {
  icon: string;
  glyphClass?: string;
  title: string;
  description: string;
  children: JSX.Element;
};

function Step(props: StepProps) {
  return (
    <div class="flex flex-col items-center px-4">
      <div
        class="relative flex h-14 w-14 items-center justify-center rounded-full
               border border-gray-200 bg-white shadow-sm"
      >
        <Icon path={props.icon} size={28} class={props.glyphClass} />
      </div>
      <h2 class="mt-8 text-center text-base font-medium text-gray-800">{props.title}</h2>
      <p class="mt-1 grid min-h-8 text-center text-xs leading-4 text-gray-400">
        {props.description}
      </p>
      <div class="mt-5 flex flex-col items-center">{props.children}</div>
    </div>
  );
}

export function Home() {
  return (
    <div class="flex h-[calc(100%-2rem)] select-none items-center justify-center px-10">
      <div class="w-full max-w-3xl">
        <div class="relative grid grid-cols-3">
          <div
            aria-hidden="true"
            class="absolute top-6.75 h-0.5 bg-gray-200 left-[calc(17%+34px)] w-[calc(33%-68px)]"
          />
          <div
            aria-hidden="true"
            class="absolute top-6.75 h-0.5 bg-gray-200 left-[calc(50%+34px)] w-[calc(33%-68px)]"
          />
          <Step
            icon={mdiLaptop}
            glyphClass="text-gray-600"
            title="Select device type"
            description="Choose the type of device you want to flash"
          >
            <div class="flex w-48 flex-col gap-3">
              <button type="button" class={ghostButton}>
                <Icon path={mdiLinkVariant} size={16} class="text-gray-400" />
                Go select
              </button>
              <button type="button" class={ghostButton}>
                <Icon path={mdiFolderOpenOutline} size={16} class="text-gray-400" />
                Select local image
              </button>
            </div>
          </Step>

          <Step
            icon={mdiHarddisk}
            glyphClass="text-gray-400"
            title="Select target device"
            description="The image will be written to the selected device"
          >
            <button
              type="button"
              class="rounded-full border border-dashed border-gray-300 bg-white px-7 py-2
                     text-sm text-gray-500 shadow-sm transition-colors hover:border-gray-400
                     hover:text-gray-700 active:bg-gray-100"
            >
              Select device
            </button>
          </Step>

          <Step
            icon={mdiFlash}
            glyphClass="text-orange-500"
            title="Start flashing"
            description="Verify everything and start writing"
          >
            <button
              type="button"
              class="inline-flex items-center gap-2 rounded-full bg-blue-500 px-8 py-2
                     text-sm font-medium text-white shadow-md transition-colors
                     hover:bg-blue-600 active:bg-blue-700"
            >
              <Icon path={mdiFlash} size={16} />
              Flash!
            </button>
          </Step>
        </div>
      </div>
    </div>
  );
}
