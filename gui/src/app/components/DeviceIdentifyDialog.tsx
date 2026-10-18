import { For, Show, createSignal } from "solid-js";
import { Dialog } from "~/components/Dialog";

type DeviceIdentifyDialogProps = {
  open: boolean;
  onClose: () => void;
};

type DeviceGroup = {
  id: string;
  name: string;
  arch: "riscv64" | "arm64";
  platform: string;
  devices: string[];
};

const tabs = ["All", "RISC-V", "ARM64"] as const;

const groups: DeviceGroup[] = [
  {
    id: "jh7110",
    name: "jh7110",
    arch: "riscv64",
    platform: "riscv · sifive-u74",
    devices: ["visionfive-2", "milkv-mars", "dc-roma"],
  },
  {
    id: "eic7700",
    name: "eic7700",
    arch: "riscv64",
    platform: "riscv · sifive-p550",
    devices: ["eic7700-evb", "hifive-premier-p550", "milkv-megrez"],
  },
  {
    id: "rk3588",
    name: "rk3588",
    arch: "arm64",
    platform: "arm64 · rockchip",
    devices: ["rock-5b", "orangepi-5-plus", "cm3588"],
  },
];

export function DeviceIdentifyDialog(props: DeviceIdentifyDialogProps) {
  const [tab, setTab] = createSignal<(typeof tabs)[number]>("All");
  const [query, setQuery] = createSignal("");
  const [selected, setSelected] = createSignal("visionfive-2");

  const visibleGroups = () =>
    groups
      .filter((group) => {
        if (tab() === "RISC-V") return group.arch === "riscv64";
        if (tab() === "ARM64") return group.arch === "arm64";
        return true;
      })
      .map((group) => ({
        ...group,
        devices: group.devices.filter((device) =>
          device.toLowerCase().includes(query().trim().toLowerCase()),
        ),
      }))
      .filter((group) => group.devices.length > 0);

  return (
    <Dialog open={props.open} onClose={props.onClose}>
      <div class="flex h-full">
        <div class="flex flex-1 flex-col">
          <input
            type="text"
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
            placeholder="Search devices, e.g. visionfive-2, radxa-rock-5"
            class="overflow-hidden text-ellipsis whitespace-nowrap border-b border-gray-200
                   bg-transparent px-4 py-1.5 text-sm text-gray-700
                   placeholder:text-gray-400 focus:border-blue-400 focus:outline-none"
          />

          <div class="grid grid-cols-3">
            <For each={tabs}>
              {(item) => (
                <button
                  type="button"
                  onClick={() => setTab(item)}
                  class={`border-b py-1.5 text-sm transition-colors ${
                    tab() === item
                      ? "border-blue-500 font-medium text-blue-600"
                      : "border-gray-200 text-gray-500 hover:text-gray-700"
                  }`}
                >
                  {item}
                </button>
              )}
            </For>
          </div>

          <div class="flex-1 overflow-y-auto">
            <For each={visibleGroups()}>
              {(group) => (
                <div>
                  <h3 class="sticky top-0 z-10 bg-gray-50 px-4 py-2 text-sm font-semibold text-gray-700 shadow-sm">
                    {group.name}
                  </h3>
                  <div class="divide-y divide-gray-200">
                    <For each={group.devices}>
                      {(device) => (
                        <button
                          type="button"
                          onClick={() => setSelected(device)}
                          class={`flex w-full items-center px-4 py-2 text-left text-sm
                                  transition-colors ${
                                    selected() === device
                                      ? "bg-blue-50 font-medium text-blue-600"
                                      : "text-gray-700 hover:bg-gray-50"
                                  }`}
                        >
                          <span>{device}</span>
                        </button>
                      )}
                    </For>
                  </div>
                </div>
              )}
            </For>
            <Show when={visibleGroups().length === 0}>
              <p class="py-6 text-center text-xs text-gray-400">No matching devices.</p>
            </Show>
          </div>
        </div>
        <div class="w-px bg-gray-200" />
        <div class="flex flex-1 p-6" />
      </div>
    </Dialog>
  );
}
