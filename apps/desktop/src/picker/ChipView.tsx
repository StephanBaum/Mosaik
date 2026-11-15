/** The chip: one button at the login field of another program (ADR-0164). */
import { useState } from "react";
import { Icon } from "../components/icons.tsx";
import { LogoMark } from "../components/LogoMark.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { PickerState } from "../ipc/contract/index.ts";
import type { PickerIpc } from "./ipc.ts";

export function ChipView({ state, ipc }: { state: PickerState; ipc: PickerIpc }) {
  const { t, errorMessage } = useI18n();
  const [error, setError] = useState<string | null>(null);
  const label = t(state.locked ? "programFill.chip.locked" : "programFill.chip.label");

  const click = async (): Promise<void> => {
    const result = await ipc.call("picker_chip_click", { generation: state.generation });
    setError(result.ok ? null : errorMessage(result.error));
  };

  return (
    <button
      type="button"
      className="picker-chip"
      aria-label={label}
      title={error ?? label}
      onClick={() => void click()}
    >
      {state.locked ? <Icon name="lock" size="sm" /> : <LogoMark size="sm" />}
    </button>
  );
}
