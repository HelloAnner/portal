import { z } from "zod";

const envSchema = z.object({
  APP_ENV: z.enum(["development", "production", "test"]).default("development"),
  APP_PORT: z.string().default("8080"),
  APP_BASE_URL: z.string().default("http://localhost:8080"),
  ALLOW_PERMISSION_REQUEST: z.enum(["true", "false"]).default("true"),
});

export const env = envSchema.parse(process.env);

export const allowPermissionRequest = env.ALLOW_PERMISSION_REQUEST === "true";
