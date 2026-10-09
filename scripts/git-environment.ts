import { execFileSync } from "node:child_process";

export function gitEnvironment(): NodeJS.ProcessEnv {
  const env = { ...process.env };
  // Hooks export repository routing that would override the selected cwd.
  const localVariables = execFileSync("git", ["rev-parse", "--local-env-vars"], {
    encoding: "utf8", stdio: ["ignore", "pipe", "pipe"],
  }).trim().split("\n");
  for (const name of localVariables) delete env[name];
  return env;
}
