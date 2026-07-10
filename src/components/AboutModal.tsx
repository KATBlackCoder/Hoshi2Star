import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Copy, ExternalLink, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { toast } from "sonner";

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
  const [mounted, setMounted] = useState(isOpen);
  const [visible, setVisible] = useState(false);
  const rafRef = useRef(0);

  useEffect(() => {
    if (isOpen) {
      void getVersion().then(setVersion);
      setMounted(true);
      // Double rAF: the first guarantees the "hidden" state actually paints
      // before we flip to visible, so the CSS transition has a start frame
      // to animate from instead of jumping straight to its end state.
      const raf1 = requestAnimationFrame(() => {
        const raf2 = requestAnimationFrame(() => setVisible(true));
        rafRef.current = raf2;
      });
      rafRef.current = raf1;
      return () => cancelAnimationFrame(rafRef.current);
    }
    setVisible(false);
    const timeout = setTimeout(() => setMounted(false), 150);
    return () => clearTimeout(timeout);
  }, [isOpen]);

  if (!mounted) return null;

  return (
    <div
      className={cn(
        "fixed inset-0 z-50 flex items-center justify-center bg-black/50 transition-opacity duration-150",
        visible ? "opacity-100" : "opacity-0",
      )}
    >
      <div
        className={cn(
          "w-105 rounded-lg border bg-background p-5 shadow-xl transition-[opacity,transform] duration-150",
          visible
            ? "opacity-100 scale-100"
            : "opacity-0 scale-95 translate-y-1",
        )}
      >
        {/* Header */}
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-baseline gap-2">
            <h2 className="text-sm font-semibold">Hoshi2Star ★</h2>
            {version && (
              <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] tabular-nums text-muted-foreground">
                v{version}
              </span>
            )}
          </div>
          <button
            type="button"
            className="hit-area-40 relative active:scale-[0.96] transition-transform"
            onClick={onClose}
          >
            <X className="h-4 w-4 text-muted-foreground hover:text-foreground" />
          </button>
        </div>

        <div className="space-y-5">
          {/* Tagline + identity */}
          <section className="space-y-1.5">
            <p className="text-sm italic text-muted-foreground">
              {t("about.tagline")}
            </p>
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
      </div>
    </div>
  );
}
