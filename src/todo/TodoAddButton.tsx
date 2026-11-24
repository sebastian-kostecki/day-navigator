import { Button } from "../components";

/**
 * The per-item "Do zrobienia" button. Off the list it is a quiet "+" that adds
 * (disabled while busy or at the limit); on the list it is the filled, pressed
 * "+" that takes the item off again and stays usable at the limit.
 */
export function TodoAddButton({
  label,
  onList,
  atLimit,
  limit,
  disabled,
  disabledHint,
  onAdd,
  onRemove,
  className,
}: {
  /** Item title, for the accessible name. */
  label: string;
  onList: boolean;
  /** The list is full (or its limit is unknown). */
  atLimit: boolean;
  limit: number | null;
  /** Busy, or the list is not loaded. */
  disabled: boolean;
  /** Title shown while `disabled` and not at a known limit. */
  disabledHint?: string;
  onAdd: () => void;
  onRemove?: () => void;
  className?: string;
}) {
  if (onList) {
    return (
      <Button
        variant="primary"
        size="sm"
        iconOnly
        icon="plus"
        pressed
        className={className}
        disabled={disabled || !onRemove}
        title="Zdejmij z Do zrobienia"
        aria-label={`Zdejmij z Do zrobienia: ${label}`}
        onClick={onRemove}
      />
    );
  }
  const limitHint = limit !== null && atLimit ? `Limit ${limit} otwartych pozycji` : undefined;
  return (
    <Button
      variant="quiet"
      size="sm"
      iconOnly
      icon="plus"
      className={className}
      disabled={disabled || atLimit}
      title={limitHint ?? (disabled ? disabledHint : undefined)}
      aria-label={`Dodaj do Do zrobienia: ${label}`}
      onClick={onAdd}
    />
  );
}
