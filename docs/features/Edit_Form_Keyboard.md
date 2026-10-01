# Edit form keyboard shortcuts

As requested on 2026-09-29, data entry and edit forms use:

- Enter: move focus forward through enabled, visible controls in tab order.
- Shift+Enter: move backwards, or insert a newline in a multiline text field.
- Ctrl+Enter: activate the editor's existing Save action, retaining validation and disabled/busy checks.
- Space: activate a focused button or open the Buddhist date picker.

Focus wraps within the editor. Login and open calendar dialogs retain their existing keyboard behavior. Holding Enter does not repeat saves or focus movement; IME composition is ignored.

Coverage includes patients, drugs, regimens and drug steps, orders (including inline weight and attendance), master data, inventory entries, user/account editors, connection settings, general/guidance/printer settings and preparation details. Preparation shortcuts save details; checking and printing remain separate actions.

New editors should attach `editFormKeyboard` to `onKeyDownCapture`. Editors without a native form must mark their Save button with `data-editor-save`.
