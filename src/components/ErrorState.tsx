import { SourceState } from "./SourceState/SourceState";

type Props = {
  title: string;
  message: string;
  onRetry?: () => void;
  /** Time of the last successful refresh, shown under the message as given. */
  lastSuccess?: string;
};

/** The one place errors are rendered, so a failure never looks like an empty list. */
export function ErrorState({ title, message, onRetry, lastSuccess }: Props) {
  return (
    <SourceState
      kind="error"
      title={title}
      lastOk={lastSuccess}
      action={onRetry ? "Spróbuj ponownie" : undefined}
      onAction={onRetry}
    >
      {message}
    </SourceState>
  );
}
