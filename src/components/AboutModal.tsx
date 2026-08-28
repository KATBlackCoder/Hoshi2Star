import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Copy, ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import { HoshiyomiMark } from "@/components/brand/HoshiyomiMark";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";

const BTC_ADDRESS = "bc1qmr578evx5fzwyr754a00j9hkekd2gzpvs8zxzz";
const ETH_ADDRESS = "0x29652Fd86095913d472fF08BFEE5a15c5E7C9D51";
const GITHUB_URL = "https://github.com/KATBlackCoder/Hoshi2Star";

interface AboutModalProps {
  open: boolean;
  onClose: () => void;
}

function CopyAddressRow({
  label,
  address,
  copyLabel,
}: {
  label: string;
  address: string;
  copyLabel: string;
}) {
  async function handleCopy() {
    await navigator.clipboard.writeText(address);
    toast.success(copyLabel);
  }

  return (
    <div className="space-y-1">
      <p className="text-xs font-medium text-muted-foreground">{label}</p>
      <div className="flex items-center gap-2">
        <code className="flex-1 truncate rounded bg-muted px-2 py-1 font-mono text-[11px] text-foreground">
          {address}
        </code>
        <Button
          size="sm"
          variant="ghost"
          className="hit-area-40 relative h-7 w-7 shrink-0 p-0"
          onClick={() => void handleCopy()}
          title={copyLabel}
          aria-label={copyLabel}
        >
          <Copy className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  );
}

export function AboutModal({ open: isOpen, onClose }: AboutModalProps) {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string>("");

  useEffect(() => {
    if (isOpen) void getVersion().then(setVersion);
  }, [isOpen]);

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        className="w-[min(26rem,calc(100vw-1.5rem))]"
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          document.getElementById("about-trigger")?.focus();
        }}
      >
        {/* Header */}
        <div className="flex min-w-0 items-center justify-between pr-10">
          <div className="flex min-w-0 items-center gap-2">
            <DialogTitle className="sr-only">Hoshiyomi</DialogTitle>
            <HoshiyomiMark />
            {version && (
              <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] tabular-nums text-muted-foreground">
                v{version}
              </span>
            )}
          </div>
        </div>

        <div className="space-y-5">
          {/* Tagline + identity */}
          <section className="space-y-1.5">
            <DialogDescription className="text-sm italic text-muted-foreground">
              {t("about.tagline")}
            </DialogDescription>
            <p className="text-xs text-muted-foreground">
              {t("about.builtBy")}{" "}
              <span className="font-medium text-foreground">BlackKat</span>
              {" · "}
              <span className="font-medium text-foreground">MIT</span>
              {" · "}
              {t("about.openSource")}
            </p>
          </section>

          {/* Donate */}
          <section className="space-y-3">
            <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("about.supportTitle")}
            </h3>
            <CopyAddressRow
              label={t("about.bitcoin")}
              address={BTC_ADDRESS}
              copyLabel={t("about.copied")}
            />
            <CopyAddressRow
              label={t("about.ethereum")}
              address={ETH_ADDRESS}
              copyLabel={t("about.copied")}
            />
          </section>
        </div>

        {/* Footer */}
        <div className="mt-5 flex items-center justify-end gap-2">
          <Button
            size="sm"
            variant="ghost"
            className="h-7 gap-1.5 text-xs"
            onClick={() => void openUrl(GITHUB_URL)}
          >
            <ExternalLink className="h-3.5 w-3.5" />
            GitHub
          </Button>
          <Button
            size="sm"
            variant="outline"
            className="h-7 text-xs"
            onClick={onClose}
          >
            {t("about.close")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
