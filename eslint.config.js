import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

export default tseslint.config(
  { ignores: ["dist/", "src-tauri/"] },
  {
    files: ["src/**/*.{ts,tsx}"],
    extends: [js.configs.recommended, ...tseslint.configs.recommended, reactHooks.configs.flat.recommended],
  },
  {
    // One work-timer source for the whole app (AGENTS.md → Frontend Conventions).
    files: ["src/**/*.{ts,tsx}"],
    ignores: ["src/worktime/WorkTimerProvider.tsx"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              regex: "useWorkTimer$",
              importNames: ["useWorkTimerSource"],
              message: "Only WorkTimerProvider runs the timer source; read it with useWorkTimer().",
            },
          ],
        },
      ],
    },
  },
);
