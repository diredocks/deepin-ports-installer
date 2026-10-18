import { For, Show, createSignal } from "solid-js";
import { Dialog } from "~/components/Dialog";

type DeviceIdentifyDialogProps = {
  open: boolean;
  onClose: () => void;
};

type DeviceGroup = {
  arch: "riscv" | "arm64";
  platform: string;
  soc: string;
  devices: string[];
};

const tabs = ["All", "RISC-V", "ARM64"] as const;

const groups: DeviceGroup[] = [
  {
    arch: "riscv",
    platform: "sifive-u74",
    soc: "jh7110",
    devices: ["visionfive-2", "milkv-mars", "dc-roma"],
  },
  {
    arch: "riscv",
    platform: "sifive-p550",
    soc: "eic7700",
    devices: ["eic7700-evb", "hifive-premier-p550", "milkv-megrez", "star-pro-64", "eic7700-sbc"],
  },
  {
    arch: "riscv",
    platform: "xiuantie-c920",
    soc: "sg2042",
    devices: ["sg2042-evb", "milkv-pioneer"],
  },
  {
    arch: "riscv",
    platform: "xiuantie-c920",
    soc: "sg2044",
    devices: ["sg2044-evb"],
  },
  {
    arch: "riscv",
    platform: "xiuantie-c910",
    soc: "th1520",
    devices: ["t-head-light-a", "licheepi-4a"],
  },
  {
    arch: "riscv",
    platform: "spacemit-x60",
    soc: "k1",
    devices: [
      "muse-box",
      "muse-book",
      "muse-pi",
      "muse-pi-pro",
      "dc-roma-2",
      "licheepi-3a",
      "orangepi-rv2",
    ],
  },
  {
    arch: "riscv",
    platform: "spacemit-x100",
    soc: "k3",
    devices: ["k3-pico-itx"],
  },
  {
    arch: "riscv",
    platform: "generic",
    soc: "generic",
    devices: ["qemu-rv", "milkv-titan", "ultrarisc-dp1000", "sophgo-sg2044"],
  },
  {
    arch: "arm64",
    platform: "phytium",
    soc: "phytiumpi",
    devices: ["phytium-pi"],
  },
  {
    arch: "arm64",
    platform: "qualcomm",
    soc: "qcom",
    devices: ["radxa-dragon-q6a", "radxa-dragon-q8b"],
  },
  {
    arch: "arm64",
    platform: "rockchip",
    soc: "rockchip",
    devices: ["radxa-rock-5", "radxa-rock-4d", "dshanpi-a1", "other-rockchip-devices"],
  },
  {
    arch: "arm64",
    platform: "allwinner",
    soc: "a733",
    devices: ["radxa-cubie-a7a", "radxa-cubie-a7z"],
  },
];

export function DeviceIdentifyDialog(props: DeviceIdentifyDialogProps) {
  const [tab, setTab] = createSignal<(typeof tabs)[number]>("All");
  const [query, setQuery] = createSignal("");
  const [selected, setSelected] = createSignal("visionfive-2");

  const visibleGroups = () =>
    groups
      .filter((group) => {
        if (tab() === "RISC-V") return group.arch === "riscv";
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
                    {group.soc}
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
