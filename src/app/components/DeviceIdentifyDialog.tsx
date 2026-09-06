import { mdiChevronDown, mdiChip, mdiDevices, mdiTagMultiple } from "@mdi/js";
import { Dialog } from "~/components/Dialog";
import { Icon } from "~/components/Icon";

type DeviceIdentifyDialogProps = {
  open: boolean;
  onClose: () => void;
};

export function DeviceIdentifyDialog(props: DeviceIdentifyDialogProps) {
  return (
    <Dialog open={props.open} onClose={props.onClose}>
      <div class="flex h-full">
        <div class="flex flex-1 flex-col p-6">
          <h2 class="text-base font-medium text-gray-800">Identify your device</h2>
          <div class="mt-5 flex flex-col overflow-hidden rounded border border-gray-300">
            <div class="relative border-b border-gray-300">
              <Icon
                path={mdiChip}
                size={16}
                class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
              />
              <select
                class="w-full appearance-none bg-white py-2 pl-9 pr-8 text-sm text-gray-700
                       transition-colors hover:bg-gray-100 focus:outline-none"
              >
                <option>Architecture</option>
              </select>
              <Icon
                path={mdiChevronDown}
                size={18}
                class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400"
              />
            </div>
            <div class="relative border-b border-gray-300">
              <Icon
                path={mdiDevices}
                size={16}
                class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
              />
              <select
                class="w-full appearance-none bg-white py-2 pl-9 pr-8 text-sm text-gray-700
                       transition-colors hover:bg-gray-100 focus:outline-none"
              >
                <option>Device model</option>
              </select>
              <Icon
                path={mdiChevronDown}
                size={18}
                class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400"
              />
            </div>
            <div class="relative">
              <Icon
                path={mdiTagMultiple}
                size={16}
                class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
              />
              <select
                class="w-full appearance-none bg-white py-2 pl-9 pr-8 text-sm text-gray-700
                       transition-colors hover:bg-gray-100 focus:outline-none"
              >
                <option>Version</option>
              </select>
              <Icon
                path={mdiChevronDown}
                size={18}
                class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400"
              />
            </div>
          </div>
        </div>
        <div class="w-px bg-gray-200" />
        <div class="flex flex-1 p-6" />
      </div>
    </Dialog>
  );
}