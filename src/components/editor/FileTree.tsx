import { useTranslation } from "react-i18next";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSourceFiles } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
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
          </button>
        ))}
      </div>
    </ScrollArea>
  );
}
