import type { KeyboardEvent } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { editFormKeyboard } from "./editFormKeyboard";

function control(tagName = "INPUT", disabled = false, visible = true) {
  return { tagName, tabIndex: 0, matches: () => disabled, closest: () => null,
    getClientRects: () => visible ? [{}] : [], focus: vi.fn(), click: vi.fn() };
}

function setup() {
  vi.stubGlobal("HTMLInputElement", class {});
  vi.stubGlobal("getComputedStyle", () => ({ visibility: "visible" }));
  const first = control();
  const disabled = control("INPUT", true);
  const hidden = control("INPUT", false, false);
  const notes = control("TEXTAREA");
  const save = control("BUTTON");
  const scope = { contains: () => true, querySelector: () => save, querySelectorAll: () => [first, disabled, hidden, notes, save] };
  const event = { key: "Enter", currentTarget: scope, target: first, nativeEvent: {},
    preventDefault: vi.fn(), stopPropagation: vi.fn(), ctrlKey: false, shiftKey: false, repeat: false };
  const run = () => editFormKeyboard(event as unknown as KeyboardEvent<HTMLElement>);
  return { first, notes, save, scope, event, run };
}

afterEach(() => vi.unstubAllGlobals());

describe("edit form keyboard", () => {
  it("moves to the next visible enabled control without submitting", () => {
    const { notes, save, event, run } = setup();
    run();
    expect(notes.focus).toHaveBeenCalledOnce();
    expect(save.click).not.toHaveBeenCalled();
    expect(event.preventDefault).toHaveBeenCalledOnce();
  });
  it("moves out of a textarea on Enter, but preserves Shift+Enter for newlines", () => {
    const { notes, save, event, run } = setup();
    event.target = notes;
    event.shiftKey = true;
    run();
    expect(event.preventDefault).not.toHaveBeenCalled();
    event.shiftKey = false;
    run();
    expect(save.focus).toHaveBeenCalledOnce();
  });
  it("uses the existing save action on Ctrl+Enter, including from textarea", () => {
    const { notes, save, event, run } = setup();
    event.target = notes;
    event.ctrlKey = true;
    run();
    expect(save.click).toHaveBeenCalledOnce();
    expect(notes.focus).not.toHaveBeenCalled();
  });
  it("does not trigger a disabled save or repeat a held key", () => {
    const { save, event, run } = setup();
    event.ctrlKey = true;
    save.matches = () => true;
    run();
    save.matches = () => false;
    event.repeat = true;
    run();
    expect(save.click).not.toHaveBeenCalled();
  });
  it("leaves IME composition and portalled calendars alone", () => {
    const { scope, event, run } = setup();
    event.nativeEvent = { isComposing: true };
    run();
    event.nativeEvent = {};
    scope.contains = () => false;
    run();
    expect(event.preventDefault).not.toHaveBeenCalled();
  });
  it("moves backwards with Shift+Enter and wraps inside the editor", () => {
    const { save, event, run } = setup();
    event.shiftKey = true;
    run();
    expect(save.focus).toHaveBeenCalledOnce();
    expect(save.click).not.toHaveBeenCalled();
  });
});
