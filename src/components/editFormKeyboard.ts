import type { KeyboardEvent } from "react";

/** Shared by data editors; login/search and calendar dialogs keep their own keys. */
export function editFormKeyboard(event: KeyboardEvent<HTMLElement>) {
  if (event.key !== "Enter" || event.defaultPrevented || event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229 || event.altKey || event.metaKey) return;
  const scope = event.currentTarget;
  const target = event.target as HTMLElement;
  // React portals bubble through the editor even though the dialog is outside it.
  if (!scope.contains(target)) return;
  if (event.shiftKey && !event.ctrlKey && target.tagName === "TEXTAREA") return;

  event.preventDefault();
  event.stopPropagation();
  if (event.repeat) return;

  if (event.ctrlKey) {
    const save = scope.querySelector<HTMLElement>('[data-editor-save], button[type="submit"], button:not([type]), input[type="submit"]');
    // Click the existing action so disabled/busy states and validation still apply.
    if (save && !save.matches(':disabled, [aria-disabled="true"]')) save.click();
    return;
  }

  const controls = Array.from(scope.querySelectorAll<HTMLElement>('input, select, textarea, button, a[href], [tabindex]'))
    .filter((control) => control.tabIndex >= 0 && !control.matches(':disabled, input[type="hidden"]') && !control.closest('[hidden], [inert]') && control.getClientRects().length > 0 && getComputedStyle(control).visibility !== "hidden")
    .filter((control, _, all) => {
      if (!(control instanceof HTMLInputElement) || control.type !== "radio" || !control.name) return true;
      const group = all.filter((other) => other instanceof HTMLInputElement && other.type === "radio" && other.name === control.name) as HTMLInputElement[];
      return control === (group.find((radio) => radio.checked) ?? group[0]);
    })
    .sort((a, b) => (a.tabIndex || Infinity) - (b.tabIndex || Infinity));
  const index = controls.indexOf(target);
  if (controls.length) controls[(index + (event.shiftKey ? -1 : 1) + controls.length) % controls.length].focus();
}
