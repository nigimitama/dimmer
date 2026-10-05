import { useSyncExternalStore } from "react";

const query = window.matchMedia("(prefers-color-scheme: dark)");

const subscribe = (onChange: () => void) => {
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
};

/** OS のダークモード設定に追従する */
export const useIsDark = (): boolean => useSyncExternalStore(subscribe, () => query.matches);
