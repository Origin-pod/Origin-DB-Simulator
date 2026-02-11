import { useRef, useCallback } from 'react';

/**
 * Returns a debounced version of the callback.
 * Calls are delayed by `delay` ms; each new call resets the timer.
 */
export function useDebouncedCallback<T extends (...args: unknown[]) => void>(
  callback: T,
  delay: number,
): T {
  const timer = useRef<ReturnType<typeof setTimeout>>();

  return useCallback(
    (...args: unknown[]) => {
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => callback(...args), delay);
    },
    [callback, delay],
  ) as unknown as T;
}
