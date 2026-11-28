import { useEffect, useId, useRef, type ReactNode, type SyntheticEvent } from "react";
import "./Dialog.css";

export type DialogProps = {
  open: boolean;
  /** Asked to close (Esc). The dialog stays open until `open` turns false. */
  onClose: () => void;
  title: string;
  children?: ReactNode;
  footer?: ReactNode;
};

/** The one modal: a native `<dialog>` in the top layer that follows `open`. */
export function Dialog({ open, onClose, title, children, footer }: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  useEffect(() => {
    const dialog = ref.current;
    if (!open || !dialog) return;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.showModal();
    return () => {
      dialog.close();
      opener?.focus();
    };
  }, [open]);

  const onCancel = (event: SyntheticEvent<HTMLDialogElement>) => {
    if (event.target !== event.currentTarget) return;
    event.preventDefault();
    // Not cancelable: the webview closes anyway, and the close handler asks.
    if (event.cancelable) onClose();
  };

  // A second Esc with no other input in between closes the dialog by force. Put it
  // back and ask, so the caller still decides. The event is queued, so one that
  // follows our own close() arrives with `open` false or the dialog reopened.
  const onNativeClose = (event: SyntheticEvent<HTMLDialogElement>) => {
    const dialog = event.currentTarget;
    if (event.target !== dialog || !open || dialog.open) return;
    dialog.showModal();
    onClose();
  };

  return (
    <dialog
      ref={ref}
      className="dn-dialog"
      aria-labelledby={titleId}
      onCancel={onCancel}
      onClose={onNativeClose}
    >
      <div className="dn-dialog-head">
        <h2 id={titleId} className="dn-dialog-title">
          {title}
        </h2>
      </div>
      <div className="dn-dialog-body">{children}</div>
      {footer ? <div className="dn-dialog-foot">{footer}</div> : null}
    </dialog>
  );
}
