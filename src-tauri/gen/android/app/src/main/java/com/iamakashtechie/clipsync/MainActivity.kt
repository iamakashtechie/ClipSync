package com.iamakashtechie.clipsync

import android.Manifest
import android.content.BroadcastReceiver
import android.content.Context
import android.os.Bundle
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.core.app.ActivityCompat
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import android.content.Intent
import org.json.JSONObject

class MainActivity : TauriActivity() {
  private val prefsName = "clipsync_native_bridge"
  private val keyBackgroundModeEnabled = "background_mode_enabled"

  private val notificationPermissionReqCode = 1019
  private val nearbyPermissionReqCode = 1020
  private val keyPermissionsPrompted = "permissions_prompted"
  private var webViewRef: WebView? = null
  private var isAppForeground = true
  private var backgroundModeEnabled = true

  private val clipboardReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context?, intent: Intent?) {
      if (intent?.action != CLIPSYNC_ACTION_NATIVE_CLIPBOARD_CHANGED) {
        return
      }

      val type = intent.getStringExtra(CLIPSYNC_EXTRA_TYPE) ?: "text"
      val text = intent.getStringExtra(CLIPSYNC_EXTRA_TEXT)
      val mimeType = intent.getStringExtra(CLIPSYNC_EXTRA_MIME_TYPE)
      val imageBase64 = intent.getStringExtra(CLIPSYNC_EXTRA_IMAGE_BASE64)
      val source = intent.getStringExtra(CLIPSYNC_EXTRA_SOURCE) ?: "native"
      dispatchClipboardToWebView(type, source, text, mimeType, imageBase64)
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    val prefs = getSharedPreferences(prefsName, Context.MODE_PRIVATE)
    backgroundModeEnabled = prefs.getBoolean(keyBackgroundModeEnabled, true)
    ensureNearbyPermissions()
    applyForegroundServicePolicy()
  }

  private fun ensureNearbyPermissions() {
    val permissionsToRequest = mutableListOf<String>()

    if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
      if (ContextCompat.checkSelfPermission(this, Manifest.permission.NEARBY_WIFI_DEVICES) != PackageManager.PERMISSION_GRANTED) {
        permissionsToRequest.add(Manifest.permission.NEARBY_WIFI_DEVICES)
      }
    } else {
      if (ContextCompat.checkSelfPermission(this, Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
        permissionsToRequest.add(Manifest.permission.ACCESS_FINE_LOCATION)
      }
    }

    if (permissionsToRequest.isEmpty()) {
      return
    }

    val prefs = getSharedPreferences(prefsName, Context.MODE_PRIVATE)
    val prompted = prefs.getBoolean(keyPermissionsPrompted, false)
    if (prompted) {
      return
    }

    prefs.edit().putBoolean(keyPermissionsPrompted, true).apply()
    ActivityCompat.requestPermissions(this, permissionsToRequest.toTypedArray(), nearbyPermissionReqCode)
  }

  private fun ensureForegroundServiceStarted() {
    if (
      android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU &&
      ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) {
      ActivityCompat.requestPermissions(
        this,
        arrayOf(Manifest.permission.POST_NOTIFICATIONS),
        notificationPermissionReqCode,
      )
      return
    }

    // Keep foreground service alive for best-effort background clipboard capture.
    val serviceIntent = Intent(this, ClipSyncForegroundService::class.java)
    ContextCompat.startForegroundService(this, serviceIntent)
  }

  private fun applyForegroundServicePolicy() {
    val shouldRun = backgroundModeEnabled && !isAppForeground
    if (shouldRun) {
      ensureForegroundServiceStarted()
    } else {
      stopService(Intent(this, ClipSyncForegroundService::class.java))
    }
  }

  private fun onBackgroundModeChanged(enabled: Boolean) {
    backgroundModeEnabled = enabled
    getSharedPreferences(prefsName, Context.MODE_PRIVATE)
      .edit()
      .putBoolean(keyBackgroundModeEnabled, enabled)
      .apply()
    applyForegroundServicePolicy()
  }

  override fun onRequestPermissionsResult(
    requestCode: Int,
    permissions: Array<out String>,
    grantResults: IntArray,
  ) {
    super.onRequestPermissionsResult(requestCode, permissions, grantResults)
    if (requestCode == notificationPermissionReqCode) {
      applyForegroundServicePolicy()
      return
    }

    if (requestCode == nearbyPermissionReqCode) {
      applyForegroundServicePolicy()
    }
  }

  override fun onStart() {
    super.onStart()
    isAppForeground = true
    applyForegroundServicePolicy()
    ContextCompat.registerReceiver(
      this,
      clipboardReceiver,
      IntentFilter(CLIPSYNC_ACTION_NATIVE_CLIPBOARD_CHANGED),
      ContextCompat.RECEIVER_NOT_EXPORTED,
    )
  }

  override fun onStop() {
    isAppForeground = false
    applyForegroundServicePolicy()
    runCatching {
      unregisterReceiver(clipboardReceiver)
    }
    super.onStop()
  }

  override fun onResume() {
    super.onResume()
    isAppForeground = true
    applyForegroundServicePolicy()
    consumePendingNativeClipboard(this)?.let { payload ->
      dispatchClipboardToWebView(
        payload.type,
        payload.source,
        payload.text,
        payload.mimeType,
        payload.imageBase64,
      )
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.addJavascriptInterface(
      ClipSyncAndroidPolicyBridge { enabled -> onBackgroundModeChanged(enabled) },
      "ClipSyncAndroidPolicy",
    )
    webViewRef = webView
    consumePendingNativeClipboard(this)?.let { payload ->
      dispatchClipboardToWebView(
        payload.type,
        payload.source,
        payload.text,
        payload.mimeType,
        payload.imageBase64,
      )
    }
  }

  private fun dispatchClipboardToWebView(
    type: String,
    source: String,
    text: String?,
    mimeType: String?,
    imageBase64: String?,
  ) {
    val webView = webViewRef ?: return
    val detail = JSONObject().apply {
      put("type", type)
      put("source", source)
      put("timestampMs", System.currentTimeMillis())
      if (!text.isNullOrEmpty()) {
        put("text", text)
      }
      if (!mimeType.isNullOrEmpty()) {
        put("mimeType", mimeType)
      }
      if (!imageBase64.isNullOrEmpty()) {
        put("imageBase64", imageBase64)
      }
    }
    val js = "window.dispatchEvent(new CustomEvent('clipsync-native-clipboard', { detail: $detail }));"
    webView.evaluateJavascript(js, null)
  }
}

private class ClipSyncAndroidPolicyBridge(
  private val onModeChanged: (Boolean) -> Unit,
) {
  @JavascriptInterface
  fun setBackgroundModeEnabled(enabled: Boolean) {
    onModeChanged(enabled)
  }
}
