import { mdiClose, mdiWindowMinimize } from "@mdi/js";
import { Icon } from "~/components/Icon";

export function TitleBar() {
  return (
    <div class="h-8 flex items-center select-none">
      <div class="flex-1 h-full" style={{ "-webkit-app-region": "drag" }} />
      <div class="flex h-full" style={{ "-webkit-app-region": "no-drag" }}>
        <button
          class="w-12 h-full flex items-center justify-center
                 text-gray-500 hover:bg-gray-200 active:bg-gray-300"
        >
          <Icon path={mdiWindowMinimize} size={14} />
        </button>
        <button
          class="w-12 h-full flex items-center justify-center
                 text-gray-500 hover:bg-gray-200 active:bg-gray-300"
        >
          <Icon path={mdiClose} size={18} />
        </button>
      </div>
    </div>
  );
}
