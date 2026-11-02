/** One settings group: a title and a short description on the left, the settings on the right. */
import { useId, type ReactNode } from "react";

export function SettingsSection({
  title,
  body,
  children,
}: {
  title: string;
  body: string;
  children: ReactNode;
}) {
  const id = useId();
  return (
    <section className="settings-section" aria-labelledby={id}>
      <div className="settings-section__intro">
        <h2 id={id} className="subtitle">
          {title}
        </h2>
        <p className="caption">{body}</p>
      </div>
      <div className="settings-section__body">{children}</div>
    </section>
  );
}
