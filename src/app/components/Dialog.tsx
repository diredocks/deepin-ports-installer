import type { JSX } from "solid-js";

type DialogProps = {
  open: boolean;
  onClose: () => void;
  children?: JSX.Element;
};

export function Dialog(props: DialogProps) {
  return (
    <div
      class={`fixed inset-0 z-50 flex items-center justify-center bg-black/40 transition-opacity
              duration-200 ease-out ${props.open ? "opacity-100" : "pointer-events-none opacity-0"}`}
      aria-hidden={!props.open}
      onClick={props.onClose}
    >
      <div
        class={`h-96 w-2xl overflow-hidden rounded-lg bg-white shadow-xl transition-all duration-200 ease-out
                ${props.open ? "scale-100 opacity-100" : "scale-95 opacity-0"}`}
        onClick={(e) => e.stopPropagation()}
      >
        {props.children}
      </div>
    </div>
  );
}
