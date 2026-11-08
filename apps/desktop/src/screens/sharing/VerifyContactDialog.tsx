/** Compare a contact's safety code: the verification flow in its own dialog. */
import { Dialog } from "../../components/Dialog.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { ContactVerifier } from "./ContactVerifier.tsx";

export function VerifyContactDialog({
  email,
  onVerified,
  onClose,
}: {
  email: string;
  onVerified: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <Dialog title={t("sharing.verify.title", { email: isolate(email) })} onClose={onClose}>
      <ContactVerifier
        email={email}
        onVerified={() => {
          onVerified();
          onClose();
        }}
        onCancel={onClose}
      />
    </Dialog>
  );
}
