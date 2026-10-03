package com.hammurabicoding.nocapsnap

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.webkit.JavascriptInterface
import android.webkit.WebView
import java.util.concurrent.ConcurrentHashMap

/**
 * Tells the page whether the phone can reach the internet, for the header's Online/Offline status
 * (app/src/lib/connectivity.svelte.ts).
 *
 * The WebView's own navigator.onLine only says that some network is up. On the owner's phone (D9) a
 * VPN stayed up in airplane mode with nothing under it, and the app kept saying Online. So this asks
 * Android instead: online means a real network (Wi-Fi, mobile data, Ethernet; not a VPN) that Android
 * has validated as reaching the internet. That is the same check behind Android's own "Connected, no
 * internet", so a Wi-Fi still showing its sign-in page counts as offline too.
 *
 * The page reads CapSnapNetwork.isOnline() when it starts, and gets a "capsnap-network" event (detail:
 * true or false) on every change. It only shows a status: syncing never waits on it.
 */
class NetworkStatus(context: Context, private val webView: WebView) {
  private val connectivity = context.getSystemService(ConnectivityManager::class.java)

  // Every non-VPN network with internet that Android currently has, and whether it is validated.
  private val networks = ConcurrentHashMap<Network, Boolean>()

  @Volatile private var online = false

  private val callback = object : ConnectivityManager.NetworkCallback() {
    override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
      networks[network] = capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)
      update()
    }

    override fun onLost(network: Network) {
      networks.remove(network)
      update()
    }
  }

  inner class Bridge {
    @JavascriptInterface
    fun isOnline(): Boolean = online
  }

  /** Call before the first page load, so the page can read the status as it starts. */
  @Suppress("DEPRECATION") // allNetworks: still the one call that lists every network at once
  fun start() {
    // What is up right now, so the first read doesn't wait for the callbacks below.
    for (network in connectivity.allNetworks) {
      val capabilities = connectivity.getNetworkCapabilities(network) ?: continue
      if (capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) &&
        capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN)
      ) {
        networks[network] = capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)
      }
    }
    online = networks.containsValue(true)

    webView.addJavascriptInterface(Bridge(), "CapSnapNetwork")
    val request = NetworkRequest.Builder()
      .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
      .addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN)
      .build()
    connectivity.registerNetworkCallback(request, callback)
  }

  fun stop() {
    try {
      connectivity.unregisterNetworkCallback(callback)
    } catch (e: IllegalArgumentException) {
      // It was never registered.
    }
  }

  private fun update() {
    val now = networks.containsValue(true)
    if (now == online) return
    online = now
    webView.post {
      webView.evaluateJavascript("window.dispatchEvent(new CustomEvent('capsnap-network', { detail: $now }))", null)
    }
  }
}
