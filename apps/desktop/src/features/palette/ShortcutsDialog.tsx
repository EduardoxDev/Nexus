import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Kbd } from "@/components/ui/kbd";
import { t } from "@/lib/i18n/pt-BR";
import { useUi } from "@/stores/ui";

export function ShortcutsDialog() {
  const open = useUi((s) => s.shortcutsOpen);
  const setOpen = useUi((s) => s.setShortcutsOpen);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="p-5">
        <DialogTitle className="text-lg font-semibold">{t.shortcuts.title}</DialogTitle>
        <DialogDescription className="mt-0.5 text-sm text-fg-muted">
          {t.shortcuts.description}
        </DialogDescription>
        <ul className="mt-4 flex flex-col divide-y divide-border">
          {t.shortcuts.items.map((item) => (
            <li key={item.label} className="flex items-center justify-between py-2 text-sm">
              <span className="text-fg">{item.label}</span>
              <span className="flex gap-1">
                {item.keys.map((k) => (
                  <Kbd key={k}>{k}</Kbd>
                ))}
              </span>
            </li>
          ))}
        </ul>
      </DialogContent>
    </Dialog>
  );
}
