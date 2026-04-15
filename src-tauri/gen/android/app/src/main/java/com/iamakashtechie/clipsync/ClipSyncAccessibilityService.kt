package com.iamakashtechie.clipsync

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent

class ClipSyncAccessibilityService : AccessibilityService() {
  override fun onAccessibilityEvent(event: AccessibilityEvent?) {
    // Background clipboard-read bridge will be connected in a follow-up phase.
  }

  override fun onInterrupt() {
    // No-op for current scaffold.
  }
}
