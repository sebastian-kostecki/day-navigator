type Props = {
  title: string;
  message: string;
  onRetry?: () => void;
  /** Time of the last successful refresh, shown under the message. */
  lastSuccess?: string;
};

/** The one place errors are rendered, so a failure never looks like an empty list. */
export function ErrorState({ title, message, onRetry, lastSuccess }: Props) {
  return (
    <div className="error-state" role="alert">
      <strong>{title}</strong>
      <p>{message}</p>
      {lastSuccess && <p className="checked-at">{lastSuccess}</p>}
      {onRetry && (
        <button type="button" onClick={onRetry}>
          Spróbuj ponownie
        </button>
      )}
    </div>
  );
}
