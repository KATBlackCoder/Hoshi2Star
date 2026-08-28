import {
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ColumnDef,
} from "@tanstack/react-table";
import { Archive, Pencil } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { TerminologyEntry } from "@/lib/types";
import { useTranslation } from "react-i18next";
import { TerminologyStatusBadge } from "./TerminologyStatusBadge";

export function TerminologyTable({
  entries,
  selectedIds,
  onToggleSelected,
  onEdit,
  onArchive,
}: {
  entries: TerminologyEntry[];
  selectedIds: string[];
  onToggleSelected: (id: string) => void;
  onEdit: (entry: TerminologyEntry) => void;
  onArchive: (entry: TerminologyEntry) => void;
}) {
  const { t } = useTranslation();
  const columns: ColumnDef<TerminologyEntry>[] = [
    {
      id: "select",
      header: "",
      cell: ({ row }) => (
        <Checkbox
          aria-label={t("terminology.select", {
            term: row.original.canonicalText,
          })}
          checked={selectedIds.includes(row.original.id)}
          onCheckedChange={() => onToggleSelected(row.original.id)}
        />
      ),
    },
    {
      accessorKey: "canonicalText",
      header: t("terminology.sourceTerm"),
      cell: ({ row }) => (
        <div className="max-w-64 whitespace-normal text-safe">
          <span lang={row.original.sourceLanguage} className="font-medium leading-5">
            {row.original.canonicalText}
          </span>
          {row.original.reading && (
            <span className="ml-2 text-xs text-muted-foreground">
              {row.original.reading}
            </span>
          )}
        </div>
      ),
    },
    {
      accessorKey: "partOfSpeech",
      header: t("terminology.partOfSpeech"),
      cell: ({ getValue }) => <Tag>{String(getValue())}</Tag>,
    },
    {
      accessorKey: "semanticType",
      header: t("terminology.semanticType"),
      cell: ({ getValue }) => <Tag>{String(getValue())}</Tag>,
    },
    {
      accessorKey: "occurrenceCount",
      header: t("terminology.occurrences"),
      cell: ({ getValue }) => (
        <span className="tabular-nums">{String(getValue())}</span>
      ),
    },
    {
      id: "target",
      header: t("terminology.effectiveTarget"),
      cell: ({ row }) =>
        row.original.translation ? (
          <div className="max-w-72 whitespace-normal text-safe">
            <span lang={row.original.translation.targetLanguage} className="leading-5">
              {row.original.translation.targetText}
            </span>
            <span className="ml-2 text-xs text-muted-foreground">
              {t(
                row.original.translation.projectId
                  ? "terminology.project"
                  : "terminology.global",
              ).toLowerCase()}
            </span>
          </div>
        ) : (
          <span className="text-muted-foreground">
            {t("terminology.notTranslated")}
          </span>
        ),
    },
    {
      id: "review",
      header: t("terminology.review"),
      cell: ({ row }) => {
        const status = row.original.translation?.reviewStatus;
        return status ? (
          <TerminologyStatusBadge
            status={status}
            label={t(`terminology.reviewStatus.${status}`)}
          />
        ) : <span className="text-muted-foreground">—</span>;
      },
    },
    {
      id: "enforcement",
      header: t("terminology.enforcement"),
      cell: ({ row }) => (
        <span className="text-xs">
          {row.original.translation?.enforcement ?? "—"}
        </span>
      ),
    },
    {
      id: "actions",
      header: () => <span className="sr-only">{t("terminology.actions")}</span>,
      cell: ({ row }) => (
        <div className="flex justify-end gap-1">
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={t("terminology.edit", {
              term: row.original.canonicalText,
            })}
            onClick={() => onEdit(row.original)}
          >
            <Pencil />
          </Button>
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={t("terminology.archive", {
              term: row.original.canonicalText,
            })}
            onClick={() => onArchive(row.original)}
          >
            <Archive />
          </Button>
        </div>
      ),
    },
  ];
  const table = useReactTable({
    data: entries,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getRowId: (row) => row.id,
  });
  return (
    <div
      className="min-h-0 flex-1 overflow-auto rounded-2xl bg-card/80 shadow-[var(--shadow-surface)]"
      aria-label="Terminologie"
    >
      <Table className="min-w-[68rem] table-fixed">
        <TableHeader>
          {table.getHeaderGroups().map((group) => (
            <TableRow key={group.id}>
              {group.headers.map((header) => (
                <TableHead key={header.id}>
                  {header.isPlaceholder
                    ? null
                    : flexRender(
                        header.column.columnDef.header,
                        header.getContext(),
                      )}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows.map((row) => (
            <TableRow
              key={row.id}
              data-state={selectedIds.includes(row.id) ? "selected" : undefined}
            >
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
function Tag({ children }: { children: React.ReactNode }) {
  return (
    <span className="rounded-md bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
      {children}
    </span>
  );
}
