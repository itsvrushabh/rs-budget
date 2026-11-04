//! Progressive Web App (PWA) manifest, service worker scripts, and web shell packaging.
//!
//! Generates compliant W3C Web App Manifests (`manifest.webmanifest`), offline service worker scripts (`sw.js`),
//! and HTML index templates for hosting the 100% Rust WebAssembly client.

use serde::{Deserialize, Serialize};

/// Web App Manifest icon definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestIcon {
    pub src: String,
    pub sizes: String,
    pub r#type: String,
    pub purpose: String,
}

/// Compliant W3C Web Application Manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebManifest {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub start_url: String,
    pub display: String,
    pub background_color: String,
    pub theme_color: String,
    pub orientation: String,
    pub icons: Vec<ManifestIcon>,
}

impl Default for WebManifest {
    fn default() -> Self {
        Self {
            name: "Actual Budget".to_string(),
            short_name: "Actual".to_string(),
            description: "100% Rust local-first envelope budgeting web application".to_string(),
            start_url: "/".to_string(),
            display: "standalone".to_string(),
            background_color: "#18181b".to_string(),
            theme_color: "#2563eb".to_string(),
            orientation: "any".to_string(),
            icons: vec![
                ManifestIcon {
                    src: "/icons/icon-192.png".to_string(),
                    sizes: "192x192".to_string(),
                    r#type: "image/png".to_string(),
                    purpose: "any maskable".to_string(),
                },
                ManifestIcon {
                    src: "/icons/icon-512.png".to_string(),
                    sizes: "512x512".to_string(),
                    r#type: "image/png".to_string(),
                    purpose: "any maskable".to_string(),
                },
            ],
        }
    }
}

impl WebManifest {
    /// Serializes manifest to pretty JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Service Worker script generator for offline asset caching and OPFS sync.
pub struct ServiceWorkerBuilder {
    cache_name: String,
    static_assets: Vec<String>,
}

impl Default for ServiceWorkerBuilder {
    fn default() -> Self {
        Self {
            cache_name: "actual-budget-v1".to_string(),
            static_assets: vec![
                "/".to_string(),
                "/index.html".to_string(),
                "/actual_gui.wasm".to_string(),
                "/actual_gui.js".to_string(),
                "/manifest.webmanifest".to_string(),
                "/styles.css".to_string(),
            ],
        }
    }
}

impl ServiceWorkerBuilder {
    /// Creates a new builder with custom cache name.
    pub fn new(cache_name: impl Into<String>) -> Self {
        Self {
            cache_name: cache_name.into(),
            static_assets: Vec::new(),
        }
    }

    /// Adds a static asset to the precache list.
    pub fn add_asset(&mut self, asset: impl Into<String>) -> &mut Self {
        self.static_assets.push(asset.into());
        self
    }

    /// Generates JavaScript content for `sw.js`.
    pub fn generate_script(&self) -> String {
        let assets_json = serde_json::to_string(&self.static_assets).unwrap_or_default();
        format!(
            r#"// Auto-generated Service Worker for Actual Budget WASM PWA
const CACHE_NAME = "{cache_name}";
const STATIC_ASSETS = {assets_json};

self.addEventListener("install", (event) => {{
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {{
      return cache.addAll(STATIC_ASSETS);
    }}).then(() => self.skipWaiting())
  );
}});

self.addEventListener("activate", (event) => {{
  event.waitUntil(
    caches.keys().then((keys) => {{
      return Promise.all(
        keys.map((key) => {{
          if (key !== CACHE_NAME) {{
            return caches.delete(key);
          }}
        }})
      );
    }}).then(() => self.clients.claim())
  );
}});

self.addEventListener("fetch", (event) => {{
  const url = new URL(event.request.url);

  // Network-first strategy for sync endpoints
  if (url.pathname.startsWith("/sync/")) {{
    event.respondWith(
      fetch(event.request).catch(() => {{
        return new Response(JSON.stringify({{ status: "offline", queued: true }}), {{
          headers: {{ "Content-Type": "application/json" }},
          status: 503,
        }});
      }})
    );
    return;
  }}

  // Cache-first strategy for static assets and wasm bundles
  event.respondWith(
    caches.match(event.request).then((cached) => {{
      return (
        cached ||
        fetch(event.request).then((response) => {{
          if (!response || response.status !== 200 || response.type !== "basic") {{
            return response;
          }}
          const responseToCache = response.clone();
          caches.open(CACHE_NAME).then((cache) => {{
            cache.put(event.request, responseToCache);
          }});
          return response;
        }})
      );
    }})
  );
}});
"#,
            cache_name = self.cache_name,
            assets_json = assets_json
        )
    }
}

/// HTML shell template generator for hosting the WASM Canvas client.
pub struct IndexHtmlBuilder;

impl IndexHtmlBuilder {
    /// Generates compliant HTML5 index page.
    pub fn generate_html(title: &str, theme_color: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <meta name="theme-color" content="{theme_color}">
  <meta name="description" content="100% Rust local-first envelope budgeting">
  <link rel="manifest" href="/manifest.webmanifest">
  <title>{title}</title>
  <style>
    * {{ margin: 0; padding: 0; box-sizing: border-box; }}
    html, body {{ width: 100%; height: 100%; overflow: hidden; background-color: #18181b; color: #f4f4f5; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }}
    #app-container {{ width: 100%; height: 100%; display: flex; flex-direction: column; }}
    #budget-canvas {{ width: 100%; height: 100%; display: block; }}
    #loading-overlay {{ position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; background-color: #18181b; z-index: 100; transition: opacity 0.3s; }}
    .spinner {{ width: 40px; height: 40px; border: 4px solid #27272a; border-top-color: #2563eb; border-radius: 50%; animation: spin 1s linear infinite; }}
    @keyframes spin {{ to {{ transform: rotate(360deg); }} }}
  </style>
</head>
<body>
  <div id="app-container">
    <div id="loading-overlay">
      <div class="spinner"></div>
      <p style="margin-top: 16px; font-weight: 500;">Loading Actual Budget (WASM)...</p>
      <noscript><p style="color: #ef4444; margin-top: 8px;">JavaScript is required to run Actual Budget.</p></noscript>
    </div>
    <canvas id="budget-canvas"></canvas>
  </div>
  <script type="module">
    // Register PWA service worker
    if ('serviceWorker' in navigator) {{
      window.addEventListener('load', () => {{
        navigator.serviceWorker.register('/sw.js').catch(console.error);
      }});
    }}

    // Check Origin Private File System (OPFS) availability
    if (!navigator.storage || !navigator.storage.getDirectory) {{
      console.warn("OPFS is not supported in this browser. Falling back to memory storage.");
    }}
  </script>
</body>
</html>
"#,
            title = title,
            theme_color = theme_color
        )
    }
}
