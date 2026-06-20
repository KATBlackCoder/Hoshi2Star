import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { FontSizeDialog } from "@/components/FontSizeDialog";
import { useSourceFiles } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { cn } from "@/lib/utils";
import { formatDuration } from "@/lib/format";
import type { FontScanResult } from "@/lib/types";
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
  FlaskConical,
  Loader2,
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
  const [injectingId, setInjectingId] = useState<string | null>(null);
  const [pendingInjectId, setPendingInjectId] = useState<string | null>(null);
  const [fontScan, setFontScan] = useState<FontScanResult | null>(null);

  async function doInject(
    fileId: string,
    fontSize: number | null,
    replaceExisting: boolean,
  ) {
    setInjectingId(fileId);
    try {
      const path = await invoke<string>("debug_inject_file", {
        sourceFileId: fileId,
        fontSize,
        replaceExisting,
      });
      toast.success(t("fileTree.debugInjectDone", { path }));
    } catch (err) {
      toast.error(t("fileTree.debugInjectError", { error: String(err) }));
    } finally {
      setInjectingId(null);
    }
  }

  async function handleDebugInject(fileId: string) {
    if (injectingId) return;
    try {
      const scan = await invoke<FontScanResult>("scan_font_status", {
        sourceFileId: fileId,
      });
      setPendingInjectId(fileId);
      setFontScan(scan);
    } catch {
      // scan failure is non-fatal — inject without font dialog
      await doInject(fileId, null, false);
    }
  }

  function handleFontApply(fontSize: number, replaceExisting: boolean) {
    const fileId = pendingInjectId;
    setPendingInjectId(null);
    setFontScan(null);
    if (fileId) void doInject(fileId, fontSize, replaceExisting);
  }

  function handleFontSkip() {
    const fileId = pendingInjectId;
    setPendingInjectId(null);
    setFontScan(null);
    if (fileId) void doInject(fileId, null, false);
  }

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
    <>
      {fontScan && (
        <FontSizeDialog
          open={!!pendingInjectId}
          scan={fontScan}
          onApply={handleFontApply}
          onSkip={handleFontSkip}
        />
      )}
      <ScrollArea className="h-full">
        <div className="p-2 space-y-0.5">
          {files.map((file) => {
            const isComplete =
              file.totalCount > 0 && file.translatedCount === file.totalCount;
            return (
              <div
                key={file.id}
                role="button"
                tabIndex={0}
                onClick={() => setActiveFile(file.id)}
                onKeyDown={(e) => e.key === "Enter" && setActiveFile(file.id)}
                className={cn(
                  "group flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-xs",
                  "hover:bg-accent hover:text-accent-foreground transition-colors cursor-pointer",
                  activeFileId === file.id &&
                    "bg-accent text-accent-foreground font-medium",
                )}
              >
                {fileIcon(file.fileType)}
                <span className="truncate flex-1">{file.fileName}</span>
                {file.translationSecs !== null &&
                  file.translationSecs !== undefined && (
                    <Badge
                      variant="secondary"
                      className="shrink-0 text-[10px] opacity-70 px-1 py-0"
                    >
                      {formatDuration(file.translationSecs)}
                    </Badge>
                  )}
                {isComplete && (
                  <button
                    type="button"
                    className={cn(
                      "shrink-0 p-0.5 rounded transition-opacity text-violet-400 hover:text-violet-300",
                      "opacity-0 group-hover:opacity-100",
                    )}
                    onClick={(e) => {
                      e.stopPropagation();
                      void handleDebugInject(file.id);
                    }}
                    disabled={injectingId === file.id}
                    title={t("fileTree.debugInject")}
                  >
                    {injectingId === file.id ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      <FlaskConical className="h-3 w-3" />
                    )}
                  </button>
                )}
              </div>
            );
          })}
        </div>
      </ScrollArea>
    </>
  );
}
