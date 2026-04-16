package com.iamakashtechie.clipsync

import android.content.Context
import android.content.Intent

const val CLIPSYNC_NATIVE_PREFS = "clipsync_native_bridge"
const val CLIPSYNC_ACTION_NATIVE_CLIPBOARD_CHANGED =
  "com.iamakashtechie.clipsync.NATIVE_CLIPBOARD_CHANGED"
const val CLIPSYNC_EXTRA_TEXT = "text"
const val CLIPSYNC_EXTRA_SOURCE = "source"

private const val KEY_PENDING_TEXT = "pending_text"
private const val KEY_PENDING_SOURCE = "pending_source"
private const val KEY_LAST_DISPATCHED_TEXT = "last_dispatched_text"

private const val MAX_TEXT_LEN = 12000

fun publishNativeClipboardText(context: Context, text: String, source: String) {
  val normalized = text.trim()
  if (normalized.isEmpty()) {
    return
  }

  val clipped = if (normalized.length > MAX_TEXT_LEN) {
    normalized.take(MAX_TEXT_LEN)
  } else {
    normalized
  }

  val prefs = context.getSharedPreferences(CLIPSYNC_NATIVE_PREFS, Context.MODE_PRIVATE)
  val last = prefs.getString(KEY_LAST_DISPATCHED_TEXT, "")
  if (last == clipped) {
    return
  }

  prefs.edit()
    .putString(KEY_LAST_DISPATCHED_TEXT, clipped)
    .putString(KEY_PENDING_TEXT, clipped)
    .putString(KEY_PENDING_SOURCE, source)
    .apply()

  val intent = Intent(CLIPSYNC_ACTION_NATIVE_CLIPBOARD_CHANGED).apply {
    setPackage(context.packageName)
    putExtra(CLIPSYNC_EXTRA_TEXT, clipped)
    putExtra(CLIPSYNC_EXTRA_SOURCE, source)
  }
  context.sendBroadcast(intent)
}

fun consumePendingNativeClipboard(context: Context): Pair<String, String>? {
  val prefs = context.getSharedPreferences(CLIPSYNC_NATIVE_PREFS, Context.MODE_PRIVATE)
  val text = prefs.getString(KEY_PENDING_TEXT, null) ?: return null
  val source = prefs.getString(KEY_PENDING_SOURCE, "unknown") ?: "unknown"
  prefs.edit()
    .remove(KEY_PENDING_TEXT)
    .remove(KEY_PENDING_SOURCE)
    .apply()
  return Pair(text, source)
}