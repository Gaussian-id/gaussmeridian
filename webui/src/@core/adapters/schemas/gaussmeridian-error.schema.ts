import { z } from "zod";

// Backend emits this 3-field shape everywhere errors are constructed in
// handlers.rs/middleware.rs. NOTE: services/server/src/error.rs defines a
// separate 5-field ApiError type but it has zero call sites — do not use it
// as the contract. Some paths (e.g. bare `Err(StatusCode::X)`) return an
// EMPTY body with just a status code — callers must handle that separately,
// this schema only covers responses that DO have a JSON body.
export const GaussMeridianErrorSchema = z.object({
  error: z.object({
    message: z.string(),
    type: z.string(),
    code: z.string(),
  }),
});

export type GaussMeridianError = z.infer<typeof GaussMeridianErrorSchema>;
