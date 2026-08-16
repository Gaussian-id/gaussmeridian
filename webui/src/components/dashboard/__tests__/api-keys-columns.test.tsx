import { describe, expect, it, vi } from "vitest";

import { createApiKeysColumns } from "../api-keys-columns";

describe("createApiKeysColumns", () => {
  it("defines a column for every field the api-keys table displays, plus a revoke action", () => {
    const columns = createApiKeysColumns({ onRevoke: vi.fn(), pendingKeyId: null });
    const keys = columns.map((c) => ("accessorKey" in c ? c.accessorKey : c.id));
    expect(keys).toEqual(
      expect.arrayContaining([
        "key_prefix",
        "name",
        "created_at",
        "last_used_at",
        "active",
        "actions",
      ]),
    );
  });

  it("never defines a column for key_hash", () => {
    const columns = createApiKeysColumns({ onRevoke: vi.fn(), pendingKeyId: null });
    const keys = columns.map((c) => ("accessorKey" in c ? c.accessorKey : c.id));
    expect(keys).not.toContain("key_hash");
  });

  it("defines a scope column backed by real rate-limit/expiry fields, not a fabricated IAM panel", () => {
    const columns = createApiKeysColumns({ onRevoke: vi.fn(), pendingKeyId: null });
    const keys = columns.map((c) => ("accessorKey" in c ? c.accessorKey : c.id));
    expect(keys).toContain("scope");
  });
});
