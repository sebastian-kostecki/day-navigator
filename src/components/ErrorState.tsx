type Props = {
  title: string;
  message: string;
  onRetry?: () => void;
};

/** The one place errors are rendered, so a failure never looks like an empty list. */
export function ErrorState({ title, message, onRetry }: Props) {
  return (
    <div className="error-state" role="alert">
      <strong>{title}</strong>
      <p>{message}</p>
      {onRetry && (
        <button type="button" onClick={onRetry}>
          Spróbuj ponownie
        </button>
      )}
    </div>
  );
}
