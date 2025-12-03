import { AlertCircle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";

interface ValidationErrorsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  errors: Map<string, string>;
  onNavigateToError?: (pointer: string) => void;
}

export function ValidationErrorsDialog({
  open,
  onOpenChange,
  errors,
  onNavigateToError,
}: ValidationErrorsDialogProps) {
  const errorEntries = Array.from(errors.entries());

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[80vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertCircle className="h-5 w-5 text-destructive" />
            Validation Errors ({errors.size})
          </DialogTitle>
          <DialogDescription>
            The following fields have validation errors. Click on an error to
            navigate to the field.
          </DialogDescription>
        </DialogHeader>
        <ScrollArea className="max-h-[60vh] pr-4">
          <div className="space-y-3">
            {errorEntries.map(([pointer, message], index) => (
              <div
                key={pointer || index}
                className="rounded-lg border border-destructive/30 bg-destructive/5 p-4"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 space-y-1">
                    <p className="text-sm font-mono text-muted-foreground">
                      {pointer || "(root)"}
                    </p>
                    <p className="text-sm text-foreground">
                      {message}
                    </p>
                  </div>
                  {onNavigateToError && pointer && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        onNavigateToError(pointer);
                        onOpenChange(false);
                      }}
                    >
                      Go to field
                    </Button>
                  )}
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
