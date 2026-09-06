// TODO: select device

import {
  mdiFolderOpenOutline,
  mdiFlash,
  mdiHarddisk,
  mdiImageOutline,
  mdiLinkVariant,
} from "@mdi/js";
import { Icon } from "~/components/Icon";

type StepNodeProps = {
  path: string;
  glyphClass?: string;
  size?: number;
};

function StepNode(props: StepNodeProps) {
  return (
    <div class="flex justify-center">
      <div
        class="relative flex h-14 w-14 items-center justify-center rounded-full
               border border-gray-200 bg-white shadow-sm"
      >
        <Icon path={props.path} size={props.size ?? 28} class={props.glyphClass} />
      </div>
    </div>
  );
}

const ghostButton =
  "flex w-full items-center justify-center gap-2 rounded-full border border-gray-300 " +
  "bg-white px-4 py-2 text-sm text-gray-600 shadow-sm transition-colors " +
  "hover:border-gray-400 hover:text-gray-900 active:bg-gray-100";

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
          <StepNode path={mdiImageOutline} />
          <StepNode path={mdiHarddisk} glyphClass="text-gray-400" />
          <StepNode path={mdiFlash} glyphClass="text-orange-500" />
        </div>

        <div class="mt-8 grid grid-cols-3">
          <div class="flex flex-col items-center px-4">
            <h2 class="text-base font-medium text-gray-800">选择镜像 / 源文件</h2>
            <p class="mt-1 text-xs text-gray-400">选择镜像文件或填写下载链接</p>
            <div class="mt-5 flex w-48 flex-col gap-3">
              <button type="button" class={ghostButton}>
                <Icon path={mdiFolderOpenOutline} size={16} class="text-gray-400" />
                从文件…
              </button>
              <button type="button" class={ghostButton}>
                <Icon path={mdiLinkVariant} size={16} class="text-gray-400" />
                从链接…
              </button>
            </div>
          </div>

          <div class="flex flex-col items-center px-4">
            <h2 class="text-base font-medium text-gray-800">选择目标设备</h2>
            <p class="mt-1 text-xs text-gray-400">镜像将被写入所选设备</p>
            <button
              type="button"
              class="mt-5 rounded-full border border-dashed border-gray-300 bg-white px-7 py-2
                     text-sm text-gray-500 shadow-sm transition-colors hover:border-gray-400
                     hover:text-gray-700 active:bg-gray-100"
            >
              选择设备
            </button>
          </div>

          <div class="flex flex-col items-center px-4">
            <h2 class="text-base font-medium text-gray-800">开始烧录</h2>
            <p class="mt-1 text-xs text-gray-400">检查无误后开始写入</p>
            <button
              type="button"
              class="mt-5 inline-flex items-center gap-2 rounded-full bg-blue-500 px-8 py-2
                     text-sm font-medium text-white shadow-md transition-colors
                     hover:bg-blue-600 active:bg-blue-700"
            >
              <Icon path={mdiFlash} size={16} />
              烧录!
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
