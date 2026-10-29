import { useEffect, useRef, type RefObject } from "react";

/** Holds the latest value in a ref that is written in an effect (never during render). */
export function useLatest<T>(value: T): RefObject<T> {
  const ref = useRef(value);
  useEffect(() => {
    ref.current = value;
  }, [value]);
  return ref;
}
