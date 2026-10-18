import { mdiMagnify } from "@mdi/js";
import { For, Show, createSignal } from "solid-js";
import { Dialog } from "~/components/Dialog";
import { Icon } from "~/components/Icon";

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
  const [selected, setSelected] = createSignal("");

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
          <div class="relative">
            <Icon
              path={mdiMagnify}
              size={16}
              class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-faint"
            />
            <input
              type="text"
              value={query()}
              onInput={(e) => setQuery(e.currentTarget.value)}
              class="w-full overflow-hidden text-ellipsis whitespace-nowrap border-b border-border
                     bg-transparent py-1.5 pl-8 pr-4 text-sm text-text-secondary
                     focus:border-primary-border focus:outline-none"
            />
            <Show when={query().length === 0}>
              <span
                class="pointer-events-none absolute inset-0 truncate py-1.5 pl-8 pr-4 text-sm text-text-faint"
              >
                Search devices, e.g. visionfive-2, radxa-rock-5
              </span>
            </Show>
          </div>

          <div class="grid grid-cols-3">
            <For each={tabs}>
              {(item) => (
                <button
                  type="button"
                  onClick={() => setTab(item)}
                  class={`border-b py-1.5 text-sm transition-colors ${
                    tab() === item
                      ? "border-primary font-medium text-primary-text"
                      : "border-border text-text-soft hover:text-text-secondary"
                  }`}
                >
                  {item}
                </button>
              )}
            </For>
          </div>

          <div class="flex-1 scrollbar-overlay">
            <For each={visibleGroups()}>
              {(group) => (
                <div>
                  <h3 class="sticky top-0 z-10 bg-surface px-4 py-2 text-sm font-semibold text-text-secondary shadow-sm">
                    {group.soc}
                  </h3>
                  <div class="divide-y divide-border">
                    <For each={group.devices}>
                      {(device) => (
                        <button
                          type="button"
                          onClick={() => setSelected(device)}
                          class={`flex w-full items-center px-4 py-2 text-left text-sm
                                  transition-colors ${
                                    selected() === device
                                      ? "bg-primary-soft font-medium text-primary-text"
                                      : "text-text-secondary hover:bg-surface"
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
              <p class="py-6 text-center text-xs text-text-faint">No matching devices.</p>
            </Show>
          </div>
        </div>
        <div class="w-px bg-border" />
        <div class="flex flex-1 flex-col p-6">
          <button
            type="button"
            onClick={props.onClose}
            disabled={selected() === ""}
            class="mt-auto self-end rounded-md bg-primary px-4 py-1.5 text-sm font-medium
                   text-text-inverse shadow-sm transition-colors hover:bg-primary-hover
                   active:bg-primary-active disabled:cursor-not-allowed disabled:bg-surface-hover
                   disabled:text-text-faint disabled:shadow-none"
          >
            Confirm
          </button>
        </div>
      </div>
    </Dialog>
  );
}
