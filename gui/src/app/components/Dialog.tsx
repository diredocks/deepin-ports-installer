import { createEffect, createSignal, onCleanup, type JSX } from "solid-js";

const [dialogOpen, setDialogOpen] = createSignal(false);

export { dialogOpen };

type DialogProps = {
  open: boolean;
  onClose: () => void;
  children?: JSX.Element;
};

export function Dialog(props: DialogProps) {
  createEffect(() => setDialogOpen(props.open));
  onCleanup(() => setDialogOpen(false));

  return (
    <div
      class={`fixed inset-0 z-50 flex items-center justify-center bg-overlay transition-opacity
              duration-200 ease-out ${props.open ? "opacity-100" : "pointer-events-none opacity-0"}`}
      aria-hidden={!props.open}
      onClick={props.onClose}
    >
      <div
        class={`h-96 w-2xl overflow-hidden rounded-lg bg-canvas shadow-xl transition-all duration-200 ease-out
                ${props.open ? "scale-100 opacity-100" : "scale-95 opacity-0"}`}
        onClick={(e) => e.stopPropagation()}
      >
        {props.children}
      </div>
    </div>
  );
}
