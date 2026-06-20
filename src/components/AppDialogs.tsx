import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { SettingsModal } from "@/components/settings/SettingsModal";
import { AboutModal } from "@/components/AboutModal";
import { TranslateAllDialog } from "@/components/TranslateAllDialog";
import { FontSizeDialog } from "@/components/FontSizeDialog";
import { usePendingGlossaryExtract } from "@/stores/project";
import type { useAppHandlers } from "@/hooks/useAppHandlers";

// ---------------------------------------------------------------------------
// AppDialogs — all conditional modals and alert dialogs
// ---------------------------------------------------------------------------

type Handlers = ReturnType<typeof useAppHandlers>;

interface AppDialogsProps {
  handlers: Handlers;
}

export function AppDialogs({ handlers }: AppDialogsProps) {
  const { t } = useTranslation();
  const pendingGlossaryExtract = usePendingGlossaryExtract();

  const {
    showSettings,
    showAbout,
    exportDialog,
    exportStats,
    showTranslateAll,
    translateAllStats,
    showFontDialog,
    fontScanResult,
    setShowSettings,
    setShowAbout,
    setExportDialog,
    setShowTranslateAll,
    handleGlossaryConfirm,
    handleGlossaryDecline,
    handleExportConfirm,
    handleExportFontApply,
    handleExportFontSkip,
    handleTranslateAllStart,
  } = handlers;

  return (
    <>
      <SettingsModal
        open={showSettings}
        onClose={() => setShowSettings(false)}
      />

      <AboutModal open={showAbout} onClose={() => setShowAbout(false)} />

      <TranslateAllDialog
        open={showTranslateAll}
        segmentCount={translateAllStats?.untranslatedCount ?? 0}
        fileCount={translateAllStats?.fileCount ?? 0}
        onStart={handleTranslateAllStart}
        onCancel={() => setShowTranslateAll(false)}
      />

      {/* Export All — confirmation (all translated) */}
      <AlertDialog open={exportDialog === "confirm"}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("toolbar.exportAllConfirmTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("toolbar.exportAllConfirmDesc", {
                files: exportStats?.fileCount ?? 0,
                segments: exportStats?.totalSegments ?? 0,
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setExportDialog(null)}>
              {t("toolbar.exportAllNo")}
            </AlertDialogCancel>
            <AlertDialogAction onClick={() => void handleExportConfirm()}>
              {t("toolbar.exportAllYes")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Export All — blocked (untranslated segments remain) */}
      <AlertDialog open={exportDialog === "blocked"}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("toolbar.exportAllBlockedTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("toolbar.exportAllBlockedDesc", {
                count: exportStats?.untranslatedCount ?? 0,
                total: exportStats?.totalSegments ?? 0,
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogAction onClick={() => setExportDialog(null)}>
              {t("toolbar.exportAllClose")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Font size dialog — shown after export confirm when lines are too long */}
      {fontScanResult && (
        <FontSizeDialog
          open={showFontDialog}
          scan={fontScanResult}
          onApply={handleExportFontApply}
          onSkip={handleExportFontSkip}
        />
      )}

      {/* Glossary extraction prompt */}
      <AlertDialog open={pendingGlossaryExtract !== null}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("glossaryPrompt.title")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("glossaryPrompt.description")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={handleGlossaryDecline}>
              {t("glossaryPrompt.no")}
            </AlertDialogCancel>
            <AlertDialogAction onClick={() => void handleGlossaryConfirm()}>
              {t("glossaryPrompt.yes")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
