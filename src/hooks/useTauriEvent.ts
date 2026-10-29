import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";

/**
 * Listen to a backend event while the component is mounted. One
 * subscription per event name; each event calls the latest `handler`, so it
 * can read current props and state without re-subscribing.
 */
export function useTauriEvent<T = unknown>(
  event: string,
  handler: (payload: T) => void,
): void {
  const handlerRef = useRef(handler);
  useEffect(() => {
    handlerRef.current = handler;
  });
  useEffect(() => {
    const unlisten = listen<T>(event, (e) => handlerRef.current(e.payload));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [event]);
}
