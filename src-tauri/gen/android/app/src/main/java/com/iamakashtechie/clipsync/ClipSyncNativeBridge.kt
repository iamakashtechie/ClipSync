package com.iamakashtechie.clipsync

import android.content.Context
import android.content.Intent
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.Socket

const val CLIPSYNC_NATIVE_PREFS = "clipsync_native_bridge"
const val CLIPSYNC_ACTION_NATIVE_CLIPBOARD_CHANGED =
  "com.iamakashtechie.clipsync.NATIVE_CLIPBOARD_CHANGED"
const val CLIPSYNC_ACTION_NATIVE_RUNTIME_EVENT =
  "com.iamakashtechie.clipsync.NATIVE_RUNTIME_EVENT"
const val CLIPSYNC_EXTRA_TYPE = "type"
const val CLIPSYNC_EXTRA_TEXT = "text"
const val CLIPSYNC_EXTRA_MIME_TYPE = "mime_type"
const val CLIPSYNC_EXTRA_IMAGE_BASE64 = "image_base64"
const val CLIPSYNC_EXTRA_SOURCE = "source"
const val CLIPSYNC_EXTRA_LEVEL = "level"
const val CLIPSYNC_EXTRA_MESSAGE = "message"
const val CLIPSYNC_EXTRA_SYNC_ENABLED = "sync_enabled"

private const val NATIVE_TYPE_TEXT = "text"
private const val NATIVE_TYPE_IMAGE = "image"

private const val KEY_PENDING_TYPE = "pending_type"
private const val KEY_PENDING_TEXT = "pending_text"
private const val KEY_PENDING_MIME_TYPE = "pending_mime_type"
private const val KEY_PENDING_IMAGE_BASE64 = "pending_image_base64"
private const val KEY_PENDING_SOURCE = "pending_source"
private const val KEY_LAST_DISPATCHED_SIGNATURE = "last_dispatched_signature"
private const val KEY_PENDING_RUNTIME_LEVEL = "pending_runtime_level"
private const val KEY_PENDING_RUNTIME_MESSAGE = "pending_runtime_message"
private const val KEY_PENDING_RUNTIME_SOURCE = "pending_runtime_source"
private const val KEY_PENDING_RUNTIME_SYNC_ENABLED = "pending_runtime_sync_enabled"
private const val KEY_LAST_RUNTIME_SIGNATURE = "last_runtime_signature"

private const val MAX_TEXT_LEN = 12000
private const val MAX_IMAGE_BASE64_LEN = 8_000_000

data class NativeClipboardPayload(
  val type: String,
  val text: String?,
  val mimeType: String?,
  val imageBase64: String?,
  val source: String,
)

data class NativeRuntimeEvent(
  val level: String,
  val message: String,
  val source: String,
  val syncEnabled: Boolean?,
)

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

  val signature = "text:$clipped"
  publishNativePayload(
    context = context,
    payload = NativeClipboardPayload(
      type = NATIVE_TYPE_TEXT,
      text = clipped,
      mimeType = null,
      imageBase64 = null,
      source = source,
    ),
    signature = signature,
  )
}

fun publishNativeClipboardImage(context: Context, mimeType: String, imageBase64: String, source: String) {
  if (!mimeType.startsWith("image/")) {
    return
  }

  val normalized = imageBase64.trim()
  if (normalized.isEmpty()) {
    return
  }

  if (normalized.length > MAX_IMAGE_BASE64_LEN) {
    return
  }

  val signature = "image:$mimeType:${normalized.length}:${normalized.take(120)}"
  publishNativePayload(
    context = context,
    payload = NativeClipboardPayload(
      type = NATIVE_TYPE_IMAGE,
      text = null,
      mimeType = mimeType,
      imageBase64 = normalized,
      source = source,
    ),
    signature = signature,
  )
}

fun sendToRustLocalhost(type: String, text: String?, mimeType: String?, imageBase64: String?) {
  Thread {
    try {
      val socket = Socket("127.0.0.1", 10191)
      socket.soTimeout = 2000
      val writer = OutputStreamWriter(socket.getOutputStream(), "UTF-8")
      val json = JSONObject().apply {
        put("type", type)
        if (text != null) put("text", text)
        if (mimeType != null) put("mimeType", mimeType)
        if (imageBase64 != null) put("imageBase64", imageBase64)
      }
      writer.write(json.toString())
      writer.flush()
      writer.close()
      socket.close()
    } catch (e: Exception) {
      // Ignored: command server not started yet
    }
  }.start()
}

private fun publishNativePayload(context: Context, payload: NativeClipboardPayload, signature: String) {
  val prefs = context.getSharedPreferences(CLIPSYNC_NATIVE_PREFS, Context.MODE_PRIVATE)
  val last = prefs.getString(KEY_LAST_DISPATCHED_SIGNATURE, "")
  if (last == signature) {
    return
  }

  prefs.edit()
    .putString(KEY_LAST_DISPATCHED_SIGNATURE, signature)
    .putString(KEY_PENDING_TYPE, payload.type)
    .putString(KEY_PENDING_TEXT, payload.text)
    .putString(KEY_PENDING_MIME_TYPE, payload.mimeType)
    .putString(KEY_PENDING_IMAGE_BASE64, payload.imageBase64)
    .putString(KEY_PENDING_SOURCE, payload.source)
    .apply()

  val intent = Intent(CLIPSYNC_ACTION_NATIVE_CLIPBOARD_CHANGED).apply {
    setPackage(context.packageName)
    putExtra(CLIPSYNC_EXTRA_TYPE, payload.type)
    putExtra(CLIPSYNC_EXTRA_TEXT, payload.text)
    putExtra(CLIPSYNC_EXTRA_MIME_TYPE, payload.mimeType)
    putExtra(CLIPSYNC_EXTRA_IMAGE_BASE64, payload.imageBase64)
    putExtra(CLIPSYNC_EXTRA_SOURCE, payload.source)
  }
  context.sendBroadcast(intent)

  val isSyncEnabled = prefs.getBoolean("native_sync_enabled", true)
  if (isSyncEnabled) {
    sendToRustLocalhost(payload.type, payload.text, payload.mimeType, payload.imageBase64)
  }
}

fun consumePendingNativeClipboard(context: Context): NativeClipboardPayload? {
  val prefs = context.getSharedPreferences(CLIPSYNC_NATIVE_PREFS, Context.MODE_PRIVATE)
  val type = prefs.getString(KEY_PENDING_TYPE, null) ?: return null
  val text = prefs.getString(KEY_PENDING_TEXT, null)
  val mimeType = prefs.getString(KEY_PENDING_MIME_TYPE, null)
  val imageBase64 = prefs.getString(KEY_PENDING_IMAGE_BASE64, null)
  val source = prefs.getString(KEY_PENDING_SOURCE, "unknown") ?: "unknown"

  if (type == NATIVE_TYPE_TEXT && text.isNullOrEmpty()) {
    return null
  }
  if (type == NATIVE_TYPE_IMAGE && (mimeType.isNullOrEmpty() || imageBase64.isNullOrEmpty())) {
    return null
  }

  prefs.edit()
    .remove(KEY_PENDING_TYPE)
    .remove(KEY_PENDING_TEXT)
    .remove(KEY_PENDING_MIME_TYPE)
    .remove(KEY_PENDING_IMAGE_BASE64)
    .remove(KEY_PENDING_SOURCE)
    .apply()
  return NativeClipboardPayload(
    type = type,
    text = text,
    mimeType = mimeType,
    imageBase64 = imageBase64,
    source = source,
  )
}

fun publishNativeRuntimeEvent(
  context: Context,
  level: String,
  message: String,
  source: String,
  syncEnabled: Boolean? = null,
) {
  val normalizedLevel = level.trim().uppercase().ifEmpty { "INFO" }
  val normalizedMessage = message.trim()
  if (normalizedMessage.isEmpty()) {
    return
  }

  val normalizedSource = source.trim().ifEmpty { "native" }
  val signature = "runtime:$normalizedLevel:$normalizedSource:$normalizedMessage:${syncEnabled?.toString() ?: "none"}"
  val prefs = context.getSharedPreferences(CLIPSYNC_NATIVE_PREFS, Context.MODE_PRIVATE)
  val last = prefs.getString(KEY_LAST_RUNTIME_SIGNATURE, "")
  if (last == signature) {
    return
  }

  prefs.edit()
    .putString(KEY_LAST_RUNTIME_SIGNATURE, signature)
    .putString(KEY_PENDING_RUNTIME_LEVEL, normalizedLevel)
    .putString(KEY_PENDING_RUNTIME_MESSAGE, normalizedMessage)
    .putString(KEY_PENDING_RUNTIME_SOURCE, normalizedSource)
    .putString(KEY_PENDING_RUNTIME_SYNC_ENABLED, syncEnabled?.toString())
    .apply()

  val intent = Intent(CLIPSYNC_ACTION_NATIVE_RUNTIME_EVENT).apply {
    setPackage(context.packageName)
    putExtra(CLIPSYNC_EXTRA_LEVEL, normalizedLevel)
    putExtra(CLIPSYNC_EXTRA_MESSAGE, normalizedMessage)
    putExtra(CLIPSYNC_EXTRA_SOURCE, normalizedSource)
    if (syncEnabled != null) {
      putExtra(CLIPSYNC_EXTRA_SYNC_ENABLED, syncEnabled)
    }
  }
  context.sendBroadcast(intent)
}

fun consumePendingNativeRuntimeEvent(context: Context): NativeRuntimeEvent? {
  val prefs = context.getSharedPreferences(CLIPSYNC_NATIVE_PREFS, Context.MODE_PRIVATE)
  val level = prefs.getString(KEY_PENDING_RUNTIME_LEVEL, null) ?: return null
  val message = prefs.getString(KEY_PENDING_RUNTIME_MESSAGE, null) ?: return null
  val source = prefs.getString(KEY_PENDING_RUNTIME_SOURCE, "native") ?: "native"
  val syncEnabledRaw = prefs.getString(KEY_PENDING_RUNTIME_SYNC_ENABLED, null)
  val syncEnabled = when (syncEnabledRaw) {
    "true" -> true
    "false" -> false
    else -> null
  }

  prefs.edit()
    .remove(KEY_PENDING_RUNTIME_LEVEL)
    .remove(KEY_PENDING_RUNTIME_MESSAGE)
    .remove(KEY_PENDING_RUNTIME_SOURCE)
    .remove(KEY_PENDING_RUNTIME_SYNC_ENABLED)
    .apply()

  return NativeRuntimeEvent(level = level, message = message, source = source, syncEnabled = syncEnabled)
}