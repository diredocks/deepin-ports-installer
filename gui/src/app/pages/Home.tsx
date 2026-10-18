import { mdiFile, mdiFlash, mdiHarddisk, mdiLinkVariant, mdiPlusCircle } from "@mdi/js";
import { createSignal } from "solid-js";
import type { JSX } from "solid-js";
import { Icon } from "~/components/Icon";
import { DeviceIdentifyDialog } from "~/components/DeviceIdentifyDialog";

const ghostButton =
  "flex w-full items-center justify-center gap-2 rounded-full border border-border-strong " +
  "bg-canvas px-4 py-2 text-sm text-text-muted shadow-sm transition-colors " +
  "hover:border-border-hover hover:text-text-strong active:bg-surface-hover";

const plainButton =
  "flex w-full items-center justify-center gap-2 rounded-full bg-transparent px-4 py-2 " +
  "text-sm text-text-soft transition-colors hover:text-text-strong active:bg-surface-hover";

type StepProps = {
  icon: string;
  glyphClass?: string;
  label: string;
  children: JSX.Element;
};

function Step(props: StepProps) {
  return (
    <div class="flex flex-col items-center px-4">
      <div
        class="relative flex h-20 w-20 items-center justify-center"
        role="img"
        aria-label={props.label}
      >
        <Icon path={props.icon} size={40} class={props.glyphClass} />
      </div>
      <div class="mt-8 flex flex-col items-center">{props.children}</div>
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
            class="absolute top-9.75 h-0.5 bg-border left-[calc(17%+46px)] w-[calc(33%-92px)]"
          />
          <div
            aria-hidden="true"
            class="absolute top-9.75 h-0.5 bg-border left-[calc(50%+46px)] w-[calc(33%-92px)]"
          />
          <Step icon={mdiPlusCircle} glyphClass="text-text-muted" label="Select device type">
            <div class="flex w-40 flex-col gap-3">
              <button type="button" class={ghostButton} onClick={() => setDeviceDialogOpen(true)}>
                <Icon path={mdiLinkVariant} size={16} class="text-text-faint" />
                Select device
              </button>
              <button type="button" class={plainButton}>
                <Icon path={mdiFile} size={16} class="text-text-faint" />
                Select local file
              </button>
            </div>
          </Step>

          <Step icon={mdiHarddisk} glyphClass="text-text-faint" label="Select target device">
            <button
              type="button"
              class="w-40 rounded-full border border-dashed border-border-strong bg-canvas px-7 py-2
                     text-sm text-text-soft shadow-sm transition-colors hover:border-border-hover
                     hover:text-text-secondary active:bg-surface-hover"
            >
              Select target
            </button>
          </Step>

          <Step icon={mdiFlash} glyphClass="text-warning" label="Start flashing">
            <button
              type="button"
              class="inline-flex w-40 items-center justify-center gap-2 rounded-full bg-primary px-8 py-2
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
