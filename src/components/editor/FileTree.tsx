import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { useSourceFiles } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { cn } from "@/lib/utils";
import { formatDuration } from "@/lib/format";
import { STATUS_SUMMARY } from "@/lib/statusSummary";
import {
  FileText,
  Users,
  Sword,
  Shield,
  BookOpen,
  Skull,
  Wand,
  Map,
  List,
  Settings,
  MessageSquare,
  Database,
} from "lucide-react";

function fileIcon(fileType: string) {
  switch (fileType) {
    // --- MV/MZ file types (JSON, various colours) ---
    case "map":
      return <Map className="h-3.5 w-3.5 shrink-0 text-blue-400" />;
    case "actors":
      return <Users className="h-3.5 w-3.5 shrink-0 text-green-400" />;
    case "armors":
    case "weapons":
      return <Shield className="h-3.5 w-3.5 shrink-0 text-yellow-400" />;
    case "skills":
      return <Wand className="h-3.5 w-3.5 shrink-0 text-purple-400" />;
    case "items":
      return <Sword className="h-3.5 w-3.5 shrink-0 text-orange-400" />;
    case "enemies":
      return <Skull className="h-3.5 w-3.5 shrink-0 text-red-400" />;
    case "classes":
      return <BookOpen className="h-3.5 w-3.5 shrink-0 text-cyan-400" />;
    case "common_events":
      return <MessageSquare className="h-3.5 w-3.5 shrink-0 text-pink-400" />;
    case "map_infos":
      return <List className="h-3.5 w-3.5 shrink-0 text-indigo-400" />;
    case "system":
      return (
        <Settings className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
      );
    // --- VX Ace file types (.rvdata2, amber colour scheme) ---
    case "vx_map":
      return <Map className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_actors":
      return <Users className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_armors":
    case "vx_weapons":
      return <Shield className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_skills":
      return <Wand className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_items":
      return <Sword className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_enemies":
      return <Skull className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_classes":
      return <BookOpen className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_common_events":
      return <MessageSquare className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_map_infos":
      return <List className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    case "vx_system":
    case "vx_states":
    case "vx_troops":
      return <Settings className="h-3.5 w-3.5 shrink-0 text-amber-400" />;
    // --- Wolf RPG file types (.mps/.dat, violet colour scheme) ---
    case "wolf_map":
      return <Map className="h-3.5 w-3.5 shrink-0 text-violet-500" />;
    case "wolf_database":
      return <Database className="h-3.5 w-3.5 shrink-0 text-violet-400" />;
    default:
      return (
        <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
      );
  }
}

export function FileTree() {
  const { t } = useTranslation();
  const files = useSourceFiles();
  const activeFileId = useEditorStore((s) => s.activeFileId);
  const setActiveFile = useEditorStore((s) => s.setActiveFile);
  if (files.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-4">
        <p className="text-center text-xs text-muted-foreground leading-relaxed">
          {t("fileTree.empty")}
        </p>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto">
      <div className="p-2 space-y-0.5">
        {files.map((file) => {
          return (
            <button
              key={file.id}
              type="button"
              onClick={() => setActiveFile(file.id)}
              aria-current={activeFileId === file.id ? "true" : undefined}
              title={file.fileName}
              className={cn(
                "group flex min-h-10 w-full min-w-0 items-center gap-2 rounded px-2 py-1.5 text-left text-xs",
                "hover:bg-accent hover:text-accent-foreground transition-[background-color,color,transform] cursor-pointer active:scale-[0.96]",
                activeFileId === file.id &&
                  "bg-accent text-accent-foreground font-medium",
              )}
            >
              {fileIcon(file.fileType)}
              <span className="min-w-0 flex-1 truncate">{file.fileName}</span>
              {(file.translatedCount > 0 || file.needsReviewCount > 0) && (
                <span className="shrink-0 font-mono text-[10px] tabular-nums opacity-80">
                  {file.translatedCount > 0 && (
                    <span className={STATUS_SUMMARY.translated.className}>
                      {STATUS_SUMMARY.translated.glyph} {file.translatedCount}
                    </span>
                  )}
                  {file.translatedCount > 0 && file.needsReviewCount > 0 && (
                    <span className="text-muted-foreground"> · </span>
                  )}
                  {file.needsReviewCount > 0 && (
                    <span className={STATUS_SUMMARY.needsReview.className}>
                      {STATUS_SUMMARY.needsReview.glyph} {file.needsReviewCount}
                    </span>
                  )}
                </span>
              )}
              {file.translationSecs !== null &&
                file.translationSecs !== undefined && (
                  <Badge
                    variant="secondary"
                    className="shrink-0 text-[10px] opacity-70 px-1 py-0"
                  >
                    {formatDuration(file.translationSecs)}
                  </Badge>
                )}
            </button>
          );
        })}
      </div>
    </div>
  );
}
