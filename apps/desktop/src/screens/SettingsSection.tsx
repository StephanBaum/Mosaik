/** One settings group: a title and a short description on the left, the settings on the right. */
import { useId, type ReactNode, type Ref } from "react";

export function SettingsSection({
  title,
  body,
  headingRef,
  headingId,
  children,
}: {
  title: string;
  body: string;
  /** A fixed id for the heading, for a link that moves focus to the section. */
  headingId?: string;
  /** Makes the heading focusable by code, for a section whose content is replaced. */
  headingRef?: Ref<HTMLHeadingElement>;
  children: ReactNode;
}) {
  const generated = useId();
  const id = headingId ?? generated;
  return (
    <section className="settings-section" aria-labelledby={id}>
      <div className="settings-section__intro">
        <h2
          id={id}
          className="subtitle"
          ref={headingRef}
          tabIndex={headingRef === undefined ? undefined : -1}
        >
          {title}
        </h2>
        <p className="caption">{body}</p>
      </div>
      <div className="settings-section__body">{children}</div>
    </section>
  );
}
