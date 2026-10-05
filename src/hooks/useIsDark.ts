import { useSyncExternalStore } from "react";

const query = window.matchMedia("(prefers-color-scheme: dark)");

const subscribe = (onChange: () => void) => {
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
};

/** Follows the OS dark mode setting */
export const useIsDark = (): boolean => useSyncExternalStore(subscribe, () => query.matches);
