type IconProps = {
  path: string;
  size?: number;
  class?: string;
};

export function Icon(props: IconProps) {
  return (
    <svg
      class={props.class}
      width={props.size ?? 24}
      height={props.size ?? 24}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d={props.path} />
    </svg>
  );
}
