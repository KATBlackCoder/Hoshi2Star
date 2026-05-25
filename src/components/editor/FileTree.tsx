import { useTranslation } from "react-i18next";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { useSourceFiles } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useFileTranslationTimes } from "@/stores/llm";
import { cn } from "@/lib/utils";
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
} from "lucide-react";

function fileIcon(fileType: string) {
  switch (fileType) {
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
    default:
      return (
        <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
      );
  }
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return s > 0 ? `${m}m ${s}s` : `${m}m`;
}

export function FileTree() {
  const { t } = useTranslation();
  const files = useSourceFiles();
  const activeFileId = useEditorStore((s) => s.activeFileId);
  const setActiveFile = useEditorStore((s) => s.setActiveFile);
  const fileTranslationTimes = useFileTranslationTimes();

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
    <ScrollArea className="h-full">
      <div className="p-2 space-y-0.5">
        {files.map((file) => (
          <button
            key={file.id}
            onClick={() => setActiveFile(file.id)}
            className={cn(
              "flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-xs",
              "hover:bg-accent hover:text-accent-foreground transition-colors",
              activeFileId === file.id &&
                "bg-accent text-accent-foreground font-medium",
            )}
          >
            {fileIcon(file.fileType)}
            <span className="truncate">{file.fileName}</span>
            {fileTranslationTimes[file.id] !== undefined && (
              <Badge
                variant="secondary"
                className="ml-auto shrink-0 text-[10px] opacity-70 px-1 py-0"
              >
                {formatDuration(fileTranslationTimes[file.id])}
              </Badge>
            )}
          </button>
        ))}
      </div>
    </ScrollArea>
  );
}
