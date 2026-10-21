const POINTER_BUTTON_MASKS: Record<number, number> = {
  0: 1,
  1: 4,
  2: 2
};

export function canStartCanvasGesture(button: number): boolean {
  return button === 0 || button === 1;
}

export function isPointerButtonHeld(buttons: number, button: number): boolean {
  const mask = POINTER_BUTTON_MASKS[button] ?? 0;
  return mask !== 0 && (buttons & mask) !== 0;
}

export type PointerGestureEvent = "move" | "up" | "cancel";
export type PointerGestureAction = "ignore" | "update" | "finish" | "cancel";

export function resolvePointerGestureAction(
  activePointerId: number | null,
  eventPointerId: number,
  initiatingButton: number,
  buttons: number,
  eventType: PointerGestureEvent
): PointerGestureAction {
  if (activePointerId === null || activePointerId !== eventPointerId) return "ignore";
  if (eventType === "up") return "finish";
  if (eventType === "cancel" || !isPointerButtonHeld(buttons, initiatingButton)) return "cancel";
  return "update";
}
