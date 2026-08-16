import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

interface StatCardProps {
  label: string;
  value?: string;
  isLoading?: boolean;
}

export function StatCard({ label, value, isLoading }: StatCardProps) {
  if (isLoading) {
    return (
      <Card className="p-4">
        <Skeleton className="mb-2 h-4 w-24" />
        <Skeleton className="h-8 w-16" />
      </Card>
    );
  }

  return (
    <Card className="p-4">
      <p className="text-muted-foreground text-sm">{label}</p>
      <p className="font-display text-2xl">{value}</p>
    </Card>
  );
}
