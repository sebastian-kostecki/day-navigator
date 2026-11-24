import { Notice } from "../components";
import { ErrorState } from "../components/ErrorState";
import type { TodoPicks } from "./useTodoPicks";

function lastRefreshLabel(at: Date | null): string {
  if (at === null) return "Ostatnie udane odświeżenie: brak";
  return `Ostatnie udane odświeżenie: ${at.toLocaleTimeString("pl-PL")}`;
}

/** A source view's "Do zrobienia" state: the last action error, the limit
 * error, and a list that failed to load or to refresh. */
export function TodoPicksNotices({ todo }: { todo: TodoPicks }) {
  return (
    <>
      {todo.notice && <Notice tone="error">{todo.notice}</Notice>}
      {todo.limitError && <Notice tone="error">{todo.limitError}</Notice>}
      {todo.load.state === "error" && (
        <ErrorState
          title="Nie udało się wczytać listy Do zrobienia"
          message={todo.load.message}
          onRetry={todo.retry}
          lastSuccess={lastRefreshLabel(todo.lastSuccessAt)}
        />
      )}
      {todo.reloadError && (
        <ErrorState
          title="Nie udało się odświeżyć listy Do zrobienia — może być nieaktualna"
          message={todo.reloadError}
          onRetry={() => void todo.refresh()}
          lastSuccess={lastRefreshLabel(todo.lastSuccessAt)}
        />
      )}
    </>
  );
}
