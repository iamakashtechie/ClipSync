package com.iamakashtechie.clipsync

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent

class ClipSyncAccessibilityService : AccessibilityService() {
  override fun onAccessibilityEvent(event: AccessibilityEvent?) {
    val e = event ?: return
    if (e.isPassword) {
      return
    }

    if (
      e.eventType != AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED &&
      e.eventType != AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED &&
      e.eventType != AccessibilityEvent.TYPE_VIEW_FOCUSED
    ) {
      return
    }

    val extracted = e.text
      ?.joinToString(separator = " ") { it?.toString() ?: "" }
      ?.trim()
      .orEmpty()

    if (extracted.isEmpty()) {
      return
    }

    publishNativeClipboardText(this, extracted, "accessibility")
  }

  override fun onInterrupt() {
    // No-op.
  }
}
