package com.iamakashtechie.clipsync

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.ClipboardManager
import android.content.Context
import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.util.Base64
import java.io.ByteArrayOutputStream
import java.io.InputStream

class ClipSyncForegroundService : Service(), ClipboardManager.OnPrimaryClipChangedListener {
  private var clipboardManager: ClipboardManager? = null
  private val maxImageBytes = 2_500_000

  override fun onCreate() {
    super.onCreate()
    startForegroundInternal()
    clipboardManager = getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
    clipboardManager?.addPrimaryClipChangedListener(this)
    publishCurrentClipboard("foreground_service_start")
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    startForegroundInternal()
    publishCurrentClipboard("foreground_service_resume")
    return START_STICKY
  }

  override fun onBind(intent: Intent?): IBinder? {
    return null
  }

  override fun onDestroy() {
    clipboardManager?.removePrimaryClipChangedListener(this)
    clipboardManager = null
    super.onDestroy()
  }

  override fun onPrimaryClipChanged() {
    publishCurrentClipboard("foreground_service")
  }

  private fun publishCurrentClipboard(source: String) {
    val clip = clipboardManager?.primaryClip ?: return
    if (clip.itemCount <= 0) {
      return
    }

    val item = clip.getItemAt(0)
    val uri = item.uri
    if (uri != null) {
      val inferredMime = contentResolver.getType(uri)
        ?: clip.description?.takeIf { it.mimeTypeCount > 0 }?.getMimeType(0)

      if (inferredMime?.startsWith("image/") == true) {
        val imageBase64 = readUriAsBase64(uri, maxImageBytes)
        if (!imageBase64.isNullOrEmpty()) {
          publishNativeClipboardImage(this, inferredMime, imageBase64, source)
          return
        }
      }
    }

    val text = item.coerceToText(this)?.toString() ?: return
    publishNativeClipboardText(this, text, source)
  }

  private fun readUriAsBase64(uri: android.net.Uri, maxBytes: Int): String? {
    return try {
      contentResolver.openInputStream(uri)?.use { inputStream ->
        val bytes = readBytesLimited(inputStream, maxBytes)
        if (bytes.isEmpty()) {
          null
        } else {
          Base64.encodeToString(bytes, Base64.NO_WRAP)
        }
      }
    } catch (_: Exception) {
      null
    }
  }

  private fun readBytesLimited(inputStream: InputStream, maxBytes: Int): ByteArray {
    val output = ByteArrayOutputStream()
    val buffer = ByteArray(8192)
    var total = 0

    while (true) {
      val remaining = maxBytes - total
      if (remaining <= 0) {
        break
      }

      val toRead = if (buffer.size < remaining) buffer.size else remaining
      val read = inputStream.read(buffer, 0, toRead)
      if (read <= 0) {
        break
      }

      output.write(buffer, 0, read)
      total += read
    }

    return output.toByteArray()
  }

  private fun startForegroundInternal() {
    val channelId = "clipsync_bg_channel"

    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val channel = NotificationChannel(
        channelId,
        "ClipSync Background",
        NotificationManager.IMPORTANCE_DEFAULT,
      )
      channel.description = "Keeps ClipSync transport active in background"

      val manager = getSystemService(NotificationManager::class.java)
      manager?.createNotificationChannel(channel)
    }

    val notification = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      Notification.Builder(this, channelId)
        .setContentTitle("ClipSync")
        .setContentText("Background reliability mode active")
        .setSmallIcon(android.R.drawable.stat_notify_sync)
        .setCategory(Notification.CATEGORY_SERVICE)
        .setOngoing(true)
        .build()
    } else {
      Notification.Builder(this)
        .setContentTitle("ClipSync")
        .setContentText("Background reliability mode active")
        .setSmallIcon(android.R.drawable.stat_notify_sync)
        .setCategory(Notification.CATEGORY_SERVICE)
        .setOngoing(true)
        .build()
    }

    startForeground(1001, notification)
  }
}
