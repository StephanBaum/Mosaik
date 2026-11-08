/**
 * The heading of one step of a multi-step dialog. A step change replaces the controls that had
 * focus, which would drop focus to the page behind; the new step's heading takes it instead, so a
 * screen reader reads where the user is and the next Tab reaches the step's first control.
 */
import { useEffect, useRef, type ReactNode } from "react";

export function StepHeading({
  id,
  children,
  focus = true,
}: {
  id?: string;
  children: ReactNode;
  /** Takes focus when it appears; `false` where another control is meant to have it. */
  focus?: boolean;
}) {
  const ref = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (focus) {
      ref.current?.focus();
    }
  }, [focus]);
  return (
    <h3 ref={ref} id={id} tabIndex={-1} className="subtitle">
      {children}
    </h3>
  );
}
