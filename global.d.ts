export {};

declare global {
  interface Window {
    electron: {
      window: {
        minimize: () => void;
        close: () => void;
      };
    };
  }
}
