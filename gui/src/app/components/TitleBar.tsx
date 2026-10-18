import { mdiClose, mdiWindowMinimize } from "@mdi/js";
import { dialogOpen } from "~/components/Dialog";
import { Icon } from "~/components/Icon";
import logo from "~/assets/logo.svg";

export function TitleBar() {
  return (
    <div class="h-8 flex items-center select-none">
      <div
        class="flex-1 h-full flex items-center pl-3 text-sm font-medium text-text-muted"
        style={{ "-webkit-app-region": dialogOpen() ? "no-drag" : "drag" }}
      >
        <img src={logo} alt="Deepin logo" class="h-5 w-5" />
        <span class="ml-2">Deepin Ports Installer</span>
      </div>
      <div class="flex h-full" style={{ "-webkit-app-region": "no-drag" }}>
        <button
          class="w-12 h-full flex items-center justify-center
                 text-text-soft hover:bg-surface-active active:bg-surface-strong"
          onClick={() => window.electron.window.minimize()}
        >
          <Icon path={mdiWindowMinimize} size={14} />
        </button>
        <button
          class="w-12 h-full flex items-center justify-center
                 text-text-soft hover:bg-surface-active active:bg-surface-strong"
          onClick={() => window.electron.window.close()}
        >
          <Icon path={mdiClose} size={18} />
        </button>
      </div>
    </div>
  );
}
