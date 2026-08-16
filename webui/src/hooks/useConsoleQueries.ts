"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";

import { useDataQuery } from "@core/adapters";
import {
  MemberListSchema,
  MemberSchema,
  MoaCandidateListSchema,
  OrgListSchema,
  OrgSchema,
  OutcomeSavingsSchema,
  PermissionMatrixSchema,
  ProjectListSchema,
  ProjectSchema,
  RouteDecisionListSchema,
} from "@core/adapters/schemas/console.schema";
import type { Org, Project, Role } from "@core/adapters/schemas/console.schema";
import {
  CommerceCatalogSchema,
  CommerceInvoiceListSchema,
  CommerceSubscriptionListSchema,
  CommerceSubscriptionSchema,
  CommerceWalletSchema,
  PaymentActionSchema,
  PaymentMethodOptionsSchema,
  PaymentReceiptSchema,
  TopUpOrderListSchema,
  TopUpOrderSchema,
} from "@core/adapters/schemas/gaussmeridian.schema";
import type { TopUpOrder } from "@core/adapters/schemas/gaussmeridian.schema";
import {
  MOA_CANDIDATES_RESOURCE,
  orgMembersResource,
  orgBillingCatalogResource,
  orgBillingInvoicesResource,
  orgBillingPaymentMethodsResource,
  orgBillingSubscriptionCancelResource,
  orgBillingSubscriptionChangePlanResource,
  orgBillingSubscriptionResource,
  orgBillingSubscriptionsResource,
  orgBillingTopUpReconcileResource,
  orgBillingTopUpPaymentActionResource,
  orgBillingTopUpReceiptResource,
  orgBillingTopUpResource,
  orgBillingTopUpsResource,
  orgBillingWalletResource,
  orgProjectResource,
  orgProjectsResource,
  ORGS_RESOURCE,
  orgResource,
  orgRolesResource,
  projectRoutesResource,
  projectSavingsResource,
} from "@core/config/resources";

import { useResourceQuery } from "./useResourceQuery";

export function useOrgs() {
  return useResourceQuery({ resource: ORGS_RESOURCE, schema: OrgListSchema });
}

export function useOrg(orgId: string) {
  return useResourceQuery({
    resource: orgResource(orgId),
    schema: OrgSchema,
    enabled: Boolean(orgId),
  });
}

export function useOrgProjects(orgId: string) {
  return useResourceQuery({
    resource: orgProjectsResource(orgId),
    schema: ProjectListSchema,
    enabled: Boolean(orgId),
  });
}

export function useProject(orgId: string, projectId: string) {
  return useResourceQuery({
    resource: orgProjectResource(orgId, projectId),
    schema: ProjectSchema,
    enabled: Boolean(orgId) && Boolean(projectId),
  });
}

export function useOrgMembers(orgId: string) {
  return useResourceQuery({
    resource: orgMembersResource(orgId),
    schema: MemberListSchema,
    enabled: Boolean(orgId),
  });
}

export function usePermissionMatrix(orgId: string) {
  return useResourceQuery({
    resource: orgRolesResource(orgId),
    schema: PermissionMatrixSchema,
    enabled: Boolean(orgId),
  });
}

export function useCommerceCatalog(orgId: string) {
  return useResourceQuery({
    resource: orgBillingCatalogResource(orgId),
    schema: CommerceCatalogSchema,
    enabled: Boolean(orgId),
  });
}

export function useCommerceWallet(orgId: string) {
  return useResourceQuery({
    resource: orgBillingWalletResource(orgId),
    schema: CommerceWalletSchema,
    enabled: Boolean(orgId),
  });
}

export function useCommerceInvoices(orgId: string) {
  return useResourceQuery({
    resource: orgBillingInvoicesResource(orgId),
    schema: CommerceInvoiceListSchema,
    enabled: Boolean(orgId),
  });
}

export function useCommerceTopUps(orgId: string) {
  return useResourceQuery({
    resource: orgBillingTopUpsResource(orgId),
    schema: TopUpOrderListSchema,
    enabled: Boolean(orgId),
  });
}

const TOP_UP_POLL_BACKOFF_MS = [2_000, 4_000, 8_000, 16_000, 30_000] as const;

export function topUpPollingInterval(
  status: TopUpOrder["payment_status"] | undefined,
  completedPolls: number,
  fulfillmentStatus?: TopUpOrder["fulfillment_status"],
): number | false {
  const paymentNeedsEvidence =
    status === "created" || status === "pending" || status === "creation_unknown";
  const fulfillmentIsPending =
    status === "paid" && (fulfillmentStatus === "not_ready" || fulfillmentStatus === "ready");
  if (
    (!paymentNeedsEvidence && !fulfillmentIsPending) ||
    completedPolls >= TOP_UP_POLL_BACKOFF_MS.length
  ) {
    return false;
  }
  return TOP_UP_POLL_BACKOFF_MS[Math.max(0, completedPolls)] ?? false;
}

export function useTopUpOrder(orgId: string, orderId: string) {
  const data = useDataQuery();
  const resource = orgBillingTopUpResource(orgId, orderId);
  return useQuery({
    queryKey: [resource, null],
    queryFn: () => data.query({ resource, schema: TopUpOrderSchema }),
    enabled: Boolean(orgId) && Boolean(orderId),
    refetchInterval: (query) => {
      const status = query.state.data?.payment_status;
      const completedPolls = Math.max(
        0,
        query.state.dataUpdateCount + query.state.fetchFailureCount - 1,
      );
      return topUpPollingInterval(status, completedPolls, query.state.data?.fulfillment_status);
    },
  });
}

export function usePaymentMethods(orgId: string) {
  const data = useDataQuery();
  const resource = orgBillingPaymentMethodsResource(orgId);
  return useQuery({
    queryKey: [resource, null],
    queryFn: () => data.query({ resource, schema: PaymentMethodOptionsSchema }),
    enabled: Boolean(orgId),
    staleTime: 60_000,
  });
}

export function usePaymentAction(orgId: string, orderId: string, enabled = true) {
  const data = useDataQuery();
  const resource = orgBillingTopUpPaymentActionResource(orgId, orderId);
  return useQuery({
    queryKey: [resource, null],
    queryFn: () => data.query({ resource, schema: PaymentActionSchema }),
    enabled: enabled && Boolean(orgId) && Boolean(orderId),
    staleTime: 0,
    gcTime: 0,
    retry: false,
  });
}

export function usePaymentReceipt(orgId: string, orderId: string, enabled = true) {
  const data = useDataQuery();
  const resource = orgBillingTopUpReceiptResource(orgId, orderId);
  return useQuery({
    queryKey: [resource, null],
    queryFn: () => data.query({ resource, schema: PaymentReceiptSchema }),
    enabled: enabled && Boolean(orgId) && Boolean(orderId),
    staleTime: 0,
    retry: false,
  });
}

export function useCreateTopUp(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const resource = orgBillingTopUpsResource(orgId);
  return useMutation({
    mutationFn: (input: {
      packageId: string;
      paymentMethod: string;
      mobileNumber: string;
      idempotencyKey: string;
    }) =>
      data.query({
        resource,
        method: "POST",
        body: {
          package_id: input.packageId,
          payment_method: input.paymentMethod,
          mobile_number: input.mobileNumber,
        },
        idempotencyKey: input.idempotencyKey,
        schema: TopUpOrderSchema,
      }),
    onSuccess: (order) => {
      queryClient.setQueryData([orgBillingTopUpResource(orgId, order.order_id), null], order);
      queryClient.invalidateQueries({ queryKey: [orgBillingTopUpsResource(orgId), null] });
      queryClient.invalidateQueries({ queryKey: [orgBillingWalletResource(orgId), null] });
      queryClient.invalidateQueries({ queryKey: [orgBillingInvoicesResource(orgId), null] });
    },
  });
}

export function useReconcileTopUp(orgId: string, orderId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const resource = orgBillingTopUpReconcileResource(orgId, orderId);

  return useMutation({
    mutationFn: (input: { idempotencyKey: string }) =>
      data.query({
        resource,
        method: "POST",
        idempotencyKey: input.idempotencyKey,
        schema: TopUpOrderSchema,
      }),
    onSuccess: (order) => {
      queryClient.setQueryData([orgBillingTopUpResource(orgId, orderId), null], order);
      queryClient.invalidateQueries({ queryKey: [orgBillingTopUpsResource(orgId), null] });
      queryClient.invalidateQueries({ queryKey: [orgBillingWalletResource(orgId), null] });
      queryClient.invalidateQueries({ queryKey: [orgBillingInvoicesResource(orgId), null] });
      queryClient.invalidateQueries({
        queryKey: [orgBillingTopUpReceiptResource(orgId, orderId), null],
      });
      for (const resource of [
        "v1/balance",
        "v1/billing/budget",
        "v1/billing/summary",
        "v1/analytics/usage",
      ]) {
        queryClient.invalidateQueries({ queryKey: [resource] });
      }
    },
  });
}

export function useCommerceSubscriptions(orgId: string) {
  const data = useDataQuery();
  const resource = orgBillingSubscriptionsResource(orgId);
  return useQuery({
    queryKey: [resource, null],
    queryFn: () => data.query({ resource, schema: CommerceSubscriptionListSchema }),
    enabled: Boolean(orgId),
    refetchInterval: (query) => {
      const status = query.state.data?.[0]?.status;
      const changeStatus = query.state.data?.[0]?.pending_change?.status;
      return status === "created" ||
        status === "setup_pending" ||
        status === "creation_unknown" ||
        status === "cancellation_requested" ||
        status === "cancellation_unknown" ||
        changeStatus === "requested" ||
        changeStatus === "provider_unknown"
        ? 2_000
        : false;
    },
  });
}

export function useCommerceSubscription(orgId: string, subscriptionId: string) {
  const data = useDataQuery();
  const resource = orgBillingSubscriptionResource(orgId, subscriptionId);
  return useQuery({
    queryKey: [resource, null],
    queryFn: () => data.query({ resource, schema: CommerceSubscriptionSchema }),
    enabled: Boolean(orgId) && Boolean(subscriptionId),
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      const changeStatus = query.state.data?.pending_change?.status;
      return status === "created" ||
        status === "setup_pending" ||
        status === "creation_unknown" ||
        status === "cancellation_requested" ||
        status === "cancellation_unknown" ||
        changeStatus === "requested" ||
        changeStatus === "provider_unknown"
        ? 2_000
        : false;
    },
  });
}

export function useCreateSubscription(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const resource = orgBillingSubscriptionsResource(orgId);
  return useMutation({
    mutationFn: (input: { planId: string; idempotencyKey: string }) =>
      data.query({
        resource,
        method: "POST",
        body: { plan_id: input.planId },
        idempotencyKey: input.idempotencyKey,
        schema: CommerceSubscriptionSchema,
      }),
    onSuccess: (subscription) => {
      queryClient.setQueryData(
        [orgBillingSubscriptionResource(orgId, subscription.subscription_id), null],
        subscription,
      );
      queryClient.invalidateQueries({
        queryKey: [orgBillingSubscriptionsResource(orgId), null],
      });
    },
  });
}

export function useCancelSubscription(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (subscriptionId: string) =>
      data.query({
        resource: orgBillingSubscriptionCancelResource(orgId, subscriptionId),
        method: "POST",
        schema: CommerceSubscriptionSchema,
      }),
    onSuccess: (subscription) => {
      queryClient.setQueryData(
        [orgBillingSubscriptionResource(orgId, subscription.subscription_id), null],
        subscription,
      );
      queryClient.invalidateQueries({
        queryKey: [orgBillingSubscriptionsResource(orgId), null],
      });
    },
  });
}

export function useChangeSubscriptionPlan(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { subscriptionId: string; targetPlanId: string; idempotencyKey: string }) =>
      data.query({
        resource: orgBillingSubscriptionChangePlanResource(orgId, input.subscriptionId),
        method: "POST",
        body: { target_plan_id: input.targetPlanId },
        idempotencyKey: input.idempotencyKey,
        schema: CommerceSubscriptionSchema,
      }),
    onSuccess: (subscription) => {
      queryClient.setQueryData(
        [orgBillingSubscriptionResource(orgId, subscription.subscription_id), null],
        subscription,
      );
      queryClient.invalidateQueries({
        queryKey: [orgBillingSubscriptionsResource(orgId), null],
      });
    },
  });
}

export function useRouteDecisions(projectId: string, params?: { limit?: number }) {
  return useResourceQuery({
    resource: projectRoutesResource(projectId),
    params,
    schema: RouteDecisionListSchema,
    enabled: Boolean(projectId),
  });
}

export function useOutcomeSavings(projectId: string) {
  return useResourceQuery({
    resource: projectSavingsResource(projectId),
    schema: OutcomeSavingsSchema,
    enabled: Boolean(projectId),
  });
}

/** Fixture-backed GaussMoA candidate list for the global Playground panel (wired in M5). */
export function useMoaCandidates() {
  return useResourceQuery({ resource: MOA_CANDIDATES_RESOURCE, schema: MoaCandidateListSchema });
}

/** Creates an org. New orgs are born empty — no default project, only the creating owner. */
export function useCreateOrg() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { name: string; slug?: string; plan?: Org["plan"] }) =>
      data.query({ resource: ORGS_RESOURCE, method: "POST", body: input, schema: OrgSchema }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ORGS_RESOURCE, null] });
    },
  });
}

export function useCreateProject(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const resource = orgProjectsResource(orgId);
  return useMutation({
    mutationFn: (input: { name: string; slug?: string; environment?: Project["environment"] }) =>
      data.query({ resource, method: "POST", body: input, schema: ProjectSchema }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [resource, null] });
    },
  });
}

export function useInviteMember(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const resource = orgMembersResource(orgId);
  return useMutation({
    mutationFn: (input: { email: string; role: Role }) =>
      data.query({ resource, method: "POST", body: input, schema: MemberSchema }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [resource, null] });
    },
  });
}

export function useUpdateMemberRole(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const listResource = orgMembersResource(orgId);
  return useMutation({
    mutationFn: (input: { userId: string; role: Role }) =>
      data.query({
        resource: `${listResource}/${input.userId}`,
        method: "PATCH",
        body: { role: input.role },
        schema: MemberSchema,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [listResource, null] });
    },
  });
}

/** Removes an organization membership by the target user's canonical id. The Rust route is
 *  `/members/:uid`, where `uid` is `Member.user_id` — never the membership row id. */
export function useRemoveMember(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const listResource = orgMembersResource(orgId);
  return useMutation({
    mutationFn: (userId: string) =>
      data.query({
        resource: `${listResource}/${userId}`,
        method: "DELETE",
        schema: z.unknown(),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [listResource, null] });
    },
  });
}

export function useDeleteOrg(orgId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () =>
      data.query({ resource: orgResource(orgId), method: "DELETE", schema: z.unknown() }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ORGS_RESOURCE, null] });
    },
  });
}

/**
 * Deletes a project via `DELETE /v1/orgs/:orgId/projects/:projectId` (Admin+ — real backend
 * route, `handlers.rs::delete_project`). Invalidates the org's project list so the deleted
 * project disappears without a manual refetch, matching `useDeleteOrg`'s convention.
 */
export function useDeleteProject(orgId: string, projectId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () =>
      data.query({
        resource: orgProjectResource(orgId, projectId),
        method: "DELETE",
        schema: z.unknown(),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [orgProjectsResource(orgId), null] });
    },
  });
}
