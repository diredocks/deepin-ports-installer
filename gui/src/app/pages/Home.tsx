import { mdiFlash, mdiHarddisk, mdiLinkVariant, mdiLaptop } from "@mdi/js";
import { createSignal } from "solid-js";
import type { JSX } from "solid-js";
import { Icon } from "~/components/Icon";
import { DeviceIdentifyDialog } from "~/components/DeviceIdentifyDialog";

const ghostButton =
  "flex w-full items-center justify-center gap-2 rounded-full border border-border-strong " +
  "bg-canvas px-4 py-2 text-sm text-text-muted shadow-sm transition-colors " +
  "hover:border-border-hover hover:text-text-strong active:bg-surface-hover";

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
               border border-border bg-canvas shadow-sm"
      >
        <Icon path={props.icon} size={28} class={props.glyphClass} />
      </div>
      <h2 class="mt-8 text-center text-base font-medium text-text">{props.title}</h2>
      <p class="mt-1 grid min-h-8 text-center text-xs leading-4 text-text-faint">
        {props.description}
      </p>
      <div class="mt-5 flex flex-col items-center">{props.children}</div>
    </div>
  );
}

export function Home() {
  const [deviceDialogOpen, setDeviceDialogOpen] = createSignal(false);

  return (
    <div class="flex h-[calc(100%-2rem)] select-none items-center justify-center px-10">
      <div class="w-full max-w-3xl">
        <div class="relative grid grid-cols-3">
          <div
            aria-hidden="true"
            class="absolute top-6.75 h-0.5 bg-border left-[calc(17%+34px)] w-[calc(33%-68px)]"
          />
          <div
            aria-hidden="true"
            class="absolute top-6.75 h-0.5 bg-border left-[calc(50%+34px)] w-[calc(33%-68px)]"
          />
          <Step
            icon={mdiLaptop}
            glyphClass="text-text-muted"
            title="Select device type"
            description="Choose the type of device you want to flash"
          >
            <div class="flex w-48 flex-col gap-3">
              <button type="button" class={ghostButton} onClick={() => setDeviceDialogOpen(true)}>
                <Icon path={mdiLinkVariant} size={16} class="text-text-faint" />
                Go select
              </button>
            </div>
          </Step>

          <Step
            icon={mdiHarddisk}
            glyphClass="text-text-faint"
            title="Select target device"
            description="The image will be written to the selected device"
          >
            <button
              type="button"
              class="rounded-full border border-dashed border-border-strong bg-canvas px-7 py-2
                     text-sm text-text-soft shadow-sm transition-colors hover:border-border-hover
                     hover:text-text-secondary active:bg-surface-hover"
            >
              Select device
            </button>
          </Step>

          <Step
            icon={mdiFlash}
            glyphClass="text-warning"
            title="Start flashing"
            description="Verify everything and start writing"
          >
            <button
              type="button"
              class="inline-flex items-center gap-2 rounded-full bg-primary px-8 py-2
                     text-sm font-medium text-text-inverse shadow-md transition-colors
                     hover:bg-primary-hover active:bg-primary-active"
            >
              <Icon path={mdiFlash} size={16} />
              Flash!
            </button>
          </Step>
        </div>
      </div>
      <DeviceIdentifyDialog open={deviceDialogOpen()} onClose={() => setDeviceDialogOpen(false)} />
    </div>
  );
}
