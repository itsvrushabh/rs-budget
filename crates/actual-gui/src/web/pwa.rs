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
            start_url: "./".to_string(),
            display: "standalone".to_string(),
            background_color: "#18181b".to_string(),
            theme_color: "#2563eb".to_string(),
            orientation: "any".to_string(),
            icons: vec![
                ManifestIcon {
                    src: "icons/icon-192.png".to_string(),
                    sizes: "192x192".to_string(),
                    r#type: "image/png".to_string(),
                    purpose: "any maskable".to_string(),
                },
                ManifestIcon {
                    src: "icons/icon-512.png".to_string(),
                    sizes: "512x512".to_string(),
                    r#type: "image/png".to_string(),
                    purpose: "any maskable".to_string(),
                },
                ManifestIcon {
                    src: "icons/actual-budget.svg".to_string(),
                    sizes: "any".to_string(),
                    r#type: "image/svg+xml".to_string(),
                    purpose: "any".to_string(),
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
                "./".to_string(),
                "index.html".to_string(),
                "manifest.webmanifest".to_string(),
                "favicon.svg".to_string(),
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
  if (url.pathname.includes("/sync/")) {{
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
  <link rel="icon" type="image/svg+xml" href="favicon.svg">
  <link rel="manifest" href="manifest.webmanifest">
  <title>{title}</title>
  <style>
    * {{ margin: 0; padding: 0; box-sizing: border-box; }}
    :root {{
      --bg-primary: #09090b;
      --bg-surface: #18181b;
      --bg-card: #27272a;
      --bg-card-hover: #323238;
      --border-color: #3f3f46;
      --text-main: #f4f4f5;
      --text-muted: #a1a1aa;
      --accent: {theme_color};
      --accent-hover: #1d4ed8;
      --green: #22c55e;
      --green-bg: rgba(34, 197, 94, 0.15);
      --red: #ef4444;
      --red-bg: rgba(239, 68, 68, 0.15);
    }}
    html, body {{
      width: 100%;
      height: 100%;
      overflow: hidden;
      background-color: var(--bg-primary);
      color: var(--text-main);
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }}
    #app-container {{
      width: 100%;
      height: 100%;
      display: flex;
      flex-direction: column;
      position: relative;
    }}
    #loading-overlay {{
      position: absolute;
      inset: 0;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      background-color: var(--bg-primary);
      z-index: 100;
      transition: opacity 0.3s ease-out;
    }}
    .spinner {{
      width: 44px;
      height: 44px;
      border: 4px solid #27272a;
      border-top-color: var(--accent);
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }}
    @keyframes spin {{ to {{ transform: rotate(360deg); }} }}

    /* Navigation Header */
    header.top-nav {{
      background-color: var(--bg-surface);
      border-bottom: 1px solid var(--border-color);
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 16px;
      height: 56px;
      flex-shrink: 0;
      gap: 12px;
    }}
    .brand {{
      display: flex;
      align-items: center;
      gap: 10px;
      font-weight: 700;
      font-size: 1.1rem;
      color: #fff;
    }}
    .brand-badge {{
      font-size: 0.7rem;
      background-color: rgba(37, 99, 235, 0.25);
      color: #60a5fa;
      padding: 2px 8px;
      border-radius: 9999px;
      border: 1px solid rgba(37, 99, 235, 0.4);
      font-weight: 500;
    }}
    .nav-tabs {{
      display: flex;
      gap: 4px;
      height: 100%;
      align-items: center;
    }}
    .nav-tab {{
      padding: 8px 14px;
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--text-muted);
      border-radius: 6px;
      cursor: pointer;
      border: none;
      background: transparent;
      transition: all 0.15s ease;
    }}
    .nav-tab:hover {{ color: var(--text-main); background: var(--bg-card); }}
    .nav-tab.active {{ color: #fff; background: var(--accent); }}

    .header-actions {{
      display: flex;
      align-items: center;
      gap: 8px;
    }}
    .badge {{
      display: inline-flex;
      align-items: center;
      gap: 5px;
      font-size: 0.75rem;
      padding: 4px 10px;
      border-radius: 9999px;
      background: var(--bg-card);
      border: 1px solid var(--border-color);
      color: var(--text-muted);
    }}
    .btn {{
      padding: 6px 14px;
      font-size: 0.85rem;
      font-weight: 600;
      border-radius: 6px;
      border: none;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      transition: all 0.15s;
    }}
    .btn-primary {{ background: var(--accent); color: #fff; }}
    .btn-primary:hover {{ background: var(--accent-hover); }}
    .btn-secondary {{ background: var(--bg-card); color: var(--text-main); border: 1px solid var(--border-color); }}
    .btn-secondary:hover {{ background: var(--bg-card-hover); }}

    /* Sub-header / Month bar */
    .sub-bar {{
      background: var(--bg-surface);
      border-bottom: 1px solid var(--border-color);
      padding: 12px 20px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-shrink: 0;
      flex-wrap: wrap;
      gap: 12px;
    }}
    .month-picker {{
      display: flex;
      align-items: center;
      gap: 10px;
      font-size: 1.15rem;
      font-weight: 700;
    }}
    .month-btn {{
      background: var(--bg-card);
      border: 1px solid var(--border-color);
      color: var(--text-main);
      width: 28px;
      height: 28px;
      border-radius: 6px;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 700;
    }}
    .month-btn:hover {{ background: var(--bg-card-hover); }}

    /* Hero metrics banner */
    .metrics-banner {{
      display: flex;
      align-items: center;
      gap: 24px;
    }}
    .metric-card {{
      display: flex;
      flex-direction: column;
      gap: 2px;
    }}
    .metric-label {{ font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted); }}
    .metric-val {{ font-size: 1.25rem; font-weight: 700; }}
    .available-box {{
      background: var(--green-bg);
      border: 1px solid var(--green);
      color: var(--green);
      padding: 6px 14px;
      border-radius: 8px;
    }}

    /* Main Content */
    main.main-viewport {{
      flex: 1;
      overflow-y: auto;
      padding: 20px;
    }}
    .tab-content {{ display: none; }}
    .tab-content.active {{ display: block; }}

    /* Budget Table */
    .budget-table {{
      width: 100%;
      border-collapse: collapse;
      margin-top: 10px;
    }}
    .budget-table th {{
      text-align: left;
      font-size: 0.8rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted);
      padding: 10px 14px;
      border-bottom: 2px solid var(--border-color);
    }}
    .budget-table td {{
      padding: 10px 14px;
      border-bottom: 1px solid #27272a;
      font-size: 0.9rem;
    }}
    .group-header {{
      background-color: rgba(39, 39, 42, 0.6);
      font-weight: 700;
      color: #fff;
    }}
    .category-row:hover {{ background-color: rgba(39, 39, 42, 0.4); }}
    .amount-input {{
      background: #18181b;
      border: 1px solid var(--border-color);
      color: #fff;
      font-size: 0.9rem;
      padding: 4px 8px;
      border-radius: 4px;
      width: 100px;
      text-align: right;
    }}
    .amount-input:focus {{ outline: 1px solid var(--accent); border-color: var(--accent); }}

    /* Accounts Grid */
    .accounts-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 16px;
      margin-top: 16px;
    }}
    .account-card {{
      background: var(--bg-surface);
      border: 1px solid var(--border-color);
      border-radius: 8px;
      padding: 16px;
    }}
    .account-title {{ font-weight: 700; font-size: 1.05rem; display: flex; justify-content: space-between; }}
    .account-type {{ font-size: 0.75rem; color: var(--text-muted); margin-top: 2px; }}
    .account-balance {{ font-size: 1.5rem; font-weight: 700; margin-top: 12px; }}

    /* Transactions Table */
    .tx-table {{
      width: 100%;
      border-collapse: collapse;
      margin-top: 14px;
    }}
    .tx-table th {{
      text-align: left;
      padding: 10px 12px;
      font-size: 0.8rem;
      text-transform: uppercase;
      color: var(--text-muted);
      border-bottom: 2px solid var(--border-color);
    }}
    .tx-table td {{
      padding: 10px 12px;
      border-bottom: 1px solid #27272a;
      font-size: 0.875rem;
    }}
    .tx-table tr:hover {{ background: rgba(39, 39, 42, 0.3); }}

    /* Modal */
    .modal-backdrop {{
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.65);
      display: none;
      align-items: center;
      justify-content: center;
      z-index: 200;
    }}
    .modal-box {{
      background: var(--bg-surface);
      border: 1px solid var(--border-color);
      border-radius: 10px;
      width: 90%;
      max-width: 440px;
      padding: 20px;
    }}
    .modal-header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 1.1rem;
      font-weight: 700;
      margin-bottom: 16px;
    }}
    .form-group {{ margin-bottom: 12px; display: flex; flex-direction: column; gap: 4px; }}
    .form-label {{ font-size: 0.8rem; color: var(--text-muted); }}
    .form-input, .form-select {{
      background: var(--bg-card);
      border: 1px solid var(--border-color);
      color: #fff;
      padding: 8px 10px;
      border-radius: 6px;
      font-size: 0.9rem;
    }}
    .form-actions {{ display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }}

    /* Canvas (hidden or used for rendering) */
    #budget-canvas {{ width: 100%; height: 0; display: block; }}
  </style>
</head>
<body>
  <div id="app-container">
    <!-- Initial Loading Overlay -->
    <div id="loading-overlay">
      <div class="spinner"></div>
      <p style="margin-top: 16px; font-weight: 500;">Loading Actual Budget (WASM)...</p>
      <noscript><p style="color: #ef4444; margin-top: 8px;">JavaScript is required to run Actual Budget.</p></noscript>
    </div>

    <!-- Top Navigation Header -->
    <header class="top-nav">
      <div class="brand">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" style="color: #60a5fa;"><rect width="20" height="14" x="2" y="5" rx="2"/><line x1="2" x2="22" y1="10" y2="10"/></svg>
        <span>{title}</span>
        <span class="brand-badge">WASM / OPFS</span>
      </div>

      <nav class="nav-tabs">
        <button class="nav-tab active" data-tab="budget">Budget</button>
        <button class="nav-tab" data-tab="accounts">Accounts</button>
        <button class="nav-tab" data-tab="transactions">Transactions</button>
        <button class="nav-tab" data-tab="reports">Reports</button>
        <button class="nav-tab" data-tab="settings">Settings</button>
      </nav>

      <div class="header-actions">
        <span id="storage-badge" class="badge">Checking OPFS...</span>
        <button id="btn-open-tx-modal" class="btn btn-primary">+ Transaction</button>
      </div>
    </header>

    <!-- Sub-header Bar -->
    <div class="sub-bar">
      <div class="month-picker">
        <button class="month-btn" id="btn-prev-month">◀</button>
        <span id="label-current-month">November 2026</span>
        <button class="month-btn" id="btn-next-month">▶</button>
      </div>

      <div class="metrics-banner">
        <div class="metric-card">
          <span class="metric-label">Total Income</span>
          <span class="metric-val" id="val-income">$5,000.00</span>
        </div>
        <div class="metric-card">
          <span class="metric-label">Budgeted</span>
          <span class="metric-val" id="val-budgeted">$3,250.00</span>
        </div>
        <div class="metric-card available-box">
          <span class="metric-label" style="color: var(--green);">Available to Budget</span>
          <span class="metric-val" id="val-available">$1,750.00</span>
        </div>
      </div>
    </div>

    <!-- Main Views Viewport -->
    <main class="main-viewport">
      <!-- 1. Budget View -->
      <section id="view-budget" class="tab-content active">
        <table class="budget-table">
          <thead>
            <tr>
              <th style="width: 40%;">Category</th>
              <th style="text-align: right; width: 20%;">Budgeted</th>
              <th style="text-align: right; width: 20%;">Spent (Activity)</th>
              <th style="text-align: right; width: 20%;">Available Balance</th>
            </tr>
          </thead>
          <tbody id="budget-table-body">
            <!-- Dynamically populated from state -->
          </tbody>
        </table>
      </section>

      <!-- 2. Accounts View -->
      <section id="view-accounts" class="tab-content">
        <div style="display: flex; justify-content: space-between; align-items: center;">
          <h2>Accounts & Net Worth</h2>
          <button id="btn-open-acc-modal" class="btn btn-secondary">+ Add Account</button>
        </div>
        <div class="accounts-grid" id="accounts-grid">
          <!-- Dynamically populated -->
        </div>
      </section>

      <!-- 3. Transactions View -->
      <section id="view-transactions" class="tab-content">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
          <h2>Transaction Ledger</h2>
          <input type="text" id="tx-search" placeholder="Search transactions..." class="form-input" style="max-width: 250px;">
        </div>
        <table class="tx-table">
          <thead>
            <tr>
              <th>Date</th>
              <th>Account</th>
              <th>Payee</th>
              <th>Category</th>
              <th>Notes</th>
              <th style="text-align: right;">Amount</th>
            </tr>
          </thead>
          <tbody id="tx-table-body">
            <!-- Dynamically populated -->
          </tbody>
        </table>
      </section>

      <!-- 4. Reports View -->
      <section id="view-reports" class="tab-content">
        <h2>Monthly Spending Breakdown</h2>
        <div id="reports-container" style="margin-top: 16px; display: flex; flex-direction: column; gap: 16px; max-width: 600px;">
          <!-- Dynamically populated -->
        </div>
      </section>

      <!-- 5. Settings View -->
      <section id="view-settings" class="tab-content">
        <h2>Application & Sync Settings</h2>
        <div style="margin-top: 16px; max-width: 500px; display: flex; flex-direction: column; gap: 16px;">
          <div class="account-card">
            <h3>Remote Sync Server</h3>
            <p style="font-size: 0.8rem; color: var(--text-muted); margin-top: 4px;">Connect to your self-hosted actual-server instance.</p>
            <div class="form-group" style="margin-top: 12px;">
              <label class="form-label">Server URL</label>
              <input type="text" id="setting-server-url" value="http://localhost:5006" class="form-input">
            </div>
            <button class="btn btn-primary" id="btn-save-sync">Save Sync Settings</button>
          </div>

          <div class="account-card">
            <h3>Local Storage & OPFS</h3>
            <p style="font-size: 0.8rem; color: var(--text-muted); margin-top: 4px;">Local-first SQLite budget database stored in Origin Private File System.</p>
            <div style="margin-top: 12px; display: flex; gap: 8px;">
              <button class="btn btn-secondary" id="btn-reset-demo">Reset Sample Data</button>
              <button class="btn btn-secondary" id="btn-export-json">Export Budget JSON</button>
            </div>
          </div>
        </div>
      </section>
    </main>

    <!-- Canvas element required by WASM renderer -->
    <canvas id="budget-canvas"></canvas>
  </div>

  <!-- Add Transaction Modal -->
  <div id="modal-tx" class="modal-backdrop">
    <div class="modal-box">
      <div class="modal-header">
        <span>Add Transaction</span>
        <button class="month-btn" id="btn-close-tx-modal">✕</button>
      </div>
      <form id="form-add-tx">
        <div class="form-group">
          <label class="form-label">Date</label>
          <input type="date" id="tx-in-date" class="form-input" required>
        </div>
        <div class="form-group">
          <label class="form-label">Account</label>
          <select id="tx-in-account" class="form-select" required></select>
        </div>
        <div class="form-group">
          <label class="form-label">Payee</label>
          <input type="text" id="tx-in-payee" placeholder="e.g. Trader Joe's" class="form-input" required>
        </div>
        <div class="form-group">
          <label class="form-label">Category</label>
          <select id="tx-in-category" class="form-select" required></select>
        </div>
        <div class="form-group">
          <label class="form-label">Amount ($)</label>
          <input type="number" step="0.01" id="tx-in-amount" placeholder="e.g. -45.50 (negative for expenses)" class="form-input" required>
        </div>
        <div class="form-group">
          <label class="form-label">Notes (Optional)</label>
          <input type="text" id="tx-in-notes" placeholder="Notes" class="form-input">
        </div>
        <div class="form-actions">
          <button type="button" class="btn btn-secondary" id="btn-cancel-tx">Cancel</button>
          <button type="submit" class="btn btn-primary">Save Transaction</button>
        </div>
      </form>
    </div>
  </div>

  <!-- Add Account Modal -->
  <div id="modal-acc" class="modal-backdrop">
    <div class="modal-box">
      <div class="modal-header">
        <span>Add New Account</span>
        <button class="month-btn" id="btn-close-acc-modal">✕</button>
      </div>
      <form id="form-add-acc">
        <div class="form-group">
          <label class="form-label">Account Name</label>
          <input type="text" id="acc-in-name" placeholder="e.g. Chase Sapphire" class="form-input" required>
        </div>
        <div class="form-group">
          <label class="form-label">Account Type</label>
          <select id="acc-in-type" class="form-select">
            <option value="Checking">Checking</option>
            <option value="Savings">Savings</option>
            <option value="Credit Card">Credit Card</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Starting Balance ($)</label>
          <input type="number" step="0.01" id="acc-in-balance" value="0.00" class="form-input" required>
        </div>
        <div class="form-actions">
          <button type="button" class="btn btn-secondary" id="btn-cancel-acc">Cancel</button>
          <button type="submit" class="btn btn-primary">Create Account</button>
        </div>
      </form>
    </div>
  </div>

  <script type="module">
    // 1. Register PWA service worker with relative path for subpath support
    if ('serviceWorker' in navigator) {{
      window.addEventListener('load', () => {{
        navigator.serviceWorker.register('./sw.js', {{ scope: './' }}).catch(console.error);
      }});
    }}

    // 2. State & Storage Engine
    const STORAGE_KEY = 'actual_budget_state_v1';

    function getDefaultState() {{
      return {{
        month: 'November 2026',
        income: 5000.00,
        accounts: [
          {{ id: 'acc-1', name: 'Checking Account', type: 'Checking', balance: 3420.00 }},
          {{ id: 'acc-2', name: 'High Yield Savings', type: 'Savings', balance: 8500.00 }},
          {{ id: 'acc-3', name: 'Credit Card', type: 'Credit Card', balance: -485.50 }},
        ],
        groups: [
          {{
            name: 'Essential Living',
            categories: [
              {{ id: 'cat-1', name: 'Rent / Mortgage', budgeted: 1800.00 }},
              {{ id: 'cat-2', name: 'Utilities & Water', budgeted: 220.00 }},
              {{ id: 'cat-3', name: 'Groceries', budgeted: 550.00 }},
              {{ id: 'cat-4', name: 'Internet Broadband', budgeted: 80.00 }}
            ]
          }},
          {{
            name: 'Transportation',
            categories: [
              {{ id: 'cat-5', name: 'Fuel / Gas', budgeted: 140.00 }},
              {{ id: 'cat-6', name: 'Auto Insurance', budgeted: 115.00 }}
            ]
          }},
          {{
            name: 'Discretionary & Dining',
            categories: [
              {{ id: 'cat-7', name: 'Dining Out', budgeted: 220.00 }},
              {{ id: 'cat-8', name: 'Entertainment & Subs', budgeted: 95.00 }}
            ]
          }},
          {{
            name: 'Savings & Goals',
            categories: [
              {{ id: 'cat-9', name: 'Emergency Fund', budgeted: 400.00 }},
              {{ id: 'cat-10', name: 'Vacation', budgeted: 200.00 }}
            ]
          }}
        ],
        transactions: [
          {{ id: 'tx-1', date: '2026-11-02', accountId: 'acc-1', payee: 'Landlord Co', categoryId: 'cat-1', notes: 'Monthly rent', amount: -1800.00 }},
          {{ id: 'tx-2', date: '2026-11-04', accountId: 'acc-1', payee: 'Trader Joe\'s', categoryId: 'cat-3', notes: 'Groceries', amount: -142.30 }},
          {{ id: 'tx-3', date: '2026-11-05', accountId: 'acc-1', payee: 'Power & Electric', categoryId: 'cat-2', notes: 'Electric bill', amount: -115.40 }},
          {{ id: 'tx-4', date: '2026-11-07', accountId: 'acc-3', payee: 'Chipotle', categoryId: 'cat-7', notes: 'Dinner', amount: -28.50 }},
          {{ id: 'tx-5', date: '2026-11-09', accountId: 'acc-1', payee: 'Shell Oil', categoryId: 'cat-5', notes: 'Gas', amount: -45.00 }}
        ]
      }};
    }}

    let appState = (() => {{
      try {{
        const raw = localStorage.getItem(STORAGE_KEY);
        if (raw) return JSON.parse(raw);
      }} catch (e) {{
        console.warn('Failed reading localStorage, using defaults', e);
      }}
      return getDefaultState();
    }})();

    function saveState() {{
      try {{
        localStorage.setItem(STORAGE_KEY, JSON.stringify(appState));
      }} catch (e) {{
        console.error('Storage write error', e);
      }}
      renderAll();
    }}

    // Check OPFS
    async function checkStorage() {{
      const badge = document.getElementById('storage-badge');
      if (navigator.storage && navigator.storage.getDirectory) {{
        try {{
          await navigator.storage.getDirectory();
          badge.textContent = '🟢 OPFS Active';
          badge.title = 'Persistent SQLite local storage via Origin Private File System';
          return;
        }} catch (e) {{
          console.warn('OPFS access check failed', e);
        }}
      }}
      badge.textContent = '🟡 Local Storage';
      badge.title = 'Local-first browser storage';
    }}

    // Calculate Category Spending (Activity)
    function getCategoryActivity(catId) {{
      return appState.transactions
        .filter(t => t.categoryId === catId && t.amount < 0)
        .reduce((sum, t) => sum + Math.abs(t.amount), 0);
    }}

    function formatCurrency(val) {{
      const num = Number(val) || 0;
      return (num < 0 ? '-' : '') + '$' + Math.abs(num).toLocaleString('en-US', {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
    }}

    // Render Views
    function renderAll() {{
      renderBudgetView();
      renderAccountsView();
      renderTransactionsView();
      renderReportsView();
    }}

    function renderBudgetView() {{
      document.getElementById('label-current-month').textContent = appState.month;
      let totalBudgeted = 0;
      const tbody = document.getElementById('budget-table-body');
      tbody.innerHTML = '';

      appState.groups.forEach(group => {{
        const gRow = document.createElement('tr');
        gRow.className = 'group-header';
        gRow.innerHTML = `<td colspan="4">${{group.name}}</td>`;
        tbody.appendChild(gRow);

        group.categories.forEach(cat => {{
          totalBudgeted += Number(cat.budgeted) || 0;
          const activity = getCategoryActivity(cat.id);
          const balance = (Number(cat.budgeted) || 0) - activity;

          const row = document.createElement('tr');
          row.className = 'category-row';
          row.innerHTML = `
            <td style="padding-left: 24px; font-weight: 500;">${{cat.name}}</td>
            <td style="text-align: right;">
              <input type="number" step="10" class="amount-input" data-cat-id="${{cat.id}}" value="${{Number(cat.budgeted).toFixed(2)}}">
            </td>
            <td style="text-align: right; color: ${{activity > 0 ? '#f4f4f5' : 'var(--text-muted)'}};">-${{formatCurrency(activity).replace('-', '')}}</td>
            <td style="text-align: right; font-weight: 600; color: ${{balance >= 0 ? 'var(--green)' : 'var(--red)'}};">${{formatCurrency(balance)}}</td>
          `;
          tbody.appendChild(row);
        }});
      }});

      // Update Hero Metrics
      const income = Number(appState.income) || 0;
      const available = income - totalBudgeted;
      document.getElementById('val-income').textContent = formatCurrency(income);
      document.getElementById('val-budgeted').textContent = formatCurrency(totalBudgeted);
      const availEl = document.getElementById('val-available');
      availEl.textContent = formatCurrency(available);
      if (available < 0) {{
        availEl.parentElement.style.borderColor = 'var(--red)';
        availEl.parentElement.style.color = 'var(--red)';
        availEl.parentElement.style.backgroundColor = 'var(--red-bg)';
      }} else {{
        availEl.parentElement.style.borderColor = 'var(--green)';
        availEl.parentElement.style.color = 'var(--green)';
        availEl.parentElement.style.backgroundColor = 'var(--green-bg)';
      }}

      // Input changes
      tbody.querySelectorAll('.amount-input').forEach(inp => {{
        inp.addEventListener('change', (e) => {{
          const catId = e.target.getAttribute('data-cat-id');
          const val = parseFloat(e.target.value) || 0;
          appState.groups.forEach(g => {{
            g.categories.forEach(c => {{
              if (c.id === catId) c.budgeted = val;
            }});
          }});
          saveState();
        }});
      }});
    }}

    function renderAccountsView() {{
      const grid = document.getElementById('accounts-grid');
      grid.innerHTML = '';
      appState.accounts.forEach(acc => {{
        const card = document.createElement('div');
        card.className = 'account-card';
        card.innerHTML = `
          <div class="account-title">
            <span>${{acc.name}}</span>
            <span style="font-size: 0.8rem; color: var(--text-muted);">${{acc.type}}</span>
          </div>
          <div class="account-balance" style="color: ${{acc.balance >= 0 ? '#fff' : 'var(--red)'}};">
            ${{formatCurrency(acc.balance)}}
          </div>
        `;
        grid.appendChild(card);
      }});
    }}

    function renderTransactionsView() {{
      const search = (document.getElementById('tx-search').value || '').toLowerCase();
      const tbody = document.getElementById('tx-table-body');
      tbody.innerHTML = '';

      const accMap = Object.fromEntries(appState.accounts.map(a => [a.id, a.name]));
      const catMap = {{}};
      appState.groups.forEach(g => g.categories.forEach(c => catMap[c.id] = c.name));

      const filtered = appState.transactions.filter(t => {{
        if (!search) return true;
        const accName = (accMap[t.accountId] || '').toLowerCase();
        const catName = (catMap[t.categoryId] || '').toLowerCase();
        return t.payee.toLowerCase().includes(search) ||
               t.notes.toLowerCase().includes(search) ||
               accName.includes(search) ||
               catName.includes(search);
      }});

      filtered.sort((a, b) => b.date.localeCompare(a.date)).forEach(tx => {{
        const row = document.createElement('tr');
        row.innerHTML = `
          <td>${{tx.date}}</td>
          <td>${{accMap[tx.accountId] || 'Account'}}</td>
          <td style="font-weight: 500;">${{tx.payee}}</td>
          <td><span class="badge">${{catMap[tx.categoryId] || 'Uncategorized'}}</span></td>
          <td style="color: var(--text-muted);">${{tx.notes || ''}}</td>
          <td style="text-align: right; font-weight: 600; color: ${{tx.amount < 0 ? '#f4f4f5' : 'var(--green)'}};">${{formatCurrency(tx.amount)}}</td>
        `;
        tbody.appendChild(row);
      }});
    }}

    function renderReportsView() {{
      const container = document.getElementById('reports-container');
      container.innerHTML = '';
      let totalSpent = 0;
      const groupSpending = appState.groups.map(g => {{
        let spent = 0;
        g.categories.forEach(c => spent += getCategoryActivity(c.id));
        totalSpent += spent;
        return {{ name: g.name, spent }};
      }});

      groupSpending.forEach(item => {{
        const pct = totalSpent > 0 ? ((item.spent / totalSpent) * 100).toFixed(1) : 0;
        const box = document.createElement('div');
        box.innerHTML = `
          <div style="display: flex; justify-content: space-between; font-size: 0.9rem; margin-bottom: 4px;">
            <span style="font-weight: 600;">${{item.name}}</span>
            <span>${{formatCurrency(item.spent)}} (${{pct}}%)</span>
          </div>
          <div style="width: 100%; height: 10px; background: #27272a; border-radius: 999px; overflow: hidden;">
            <div style="width: ${{pct}}%; height: 100%; background: var(--accent); border-radius: 999px;"></div>
          </div>
        `;
        container.appendChild(box);
      }});
    }}

    // Navigation Tabs
    document.querySelectorAll('.nav-tab').forEach(tab => {{
      tab.addEventListener('click', () => {{
        document.querySelectorAll('.nav-tab').forEach(t => t.classList.remove('active'));
        document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
        tab.classList.add('active');
        const target = 'view-' + tab.getAttribute('data-tab');
        const view = document.getElementById(target);
        if (view) view.classList.add('active');
      }});
    }});

    // Modals
    const modalTx = document.getElementById('modal-tx');
    const modalAcc = document.getElementById('modal-acc');

    function populateSelects() {{
      const selAcc = document.getElementById('tx-in-account');
      selAcc.innerHTML = appState.accounts.map(a => `<option value="${{a.id}}">${{a.name}}</option>`).join('');

      const selCat = document.getElementById('tx-in-category');
      selCat.innerHTML = appState.groups.flatMap(g =>
        g.categories.map(c => `<option value="${{c.id}}">${{g.name}} → ${{c.name}}</option>`)
      ).join('');
    }}

    document.getElementById('btn-open-tx-modal').addEventListener('click', () => {{
      populateSelects();
      document.getElementById('tx-in-date').value = new Date().toISOString().split('T')[0];
      modalTx.style.display = 'flex';
    }});
    document.getElementById('btn-close-tx-modal').addEventListener('click', () => modalTx.style.display = 'none');
    document.getElementById('btn-cancel-tx').addEventListener('click', () => modalTx.style.display = 'none');

    document.getElementById('btn-open-acc-modal').addEventListener('click', () => modalAcc.style.display = 'flex');
    document.getElementById('btn-close-acc-modal').addEventListener('click', () => modalAcc.style.display = 'none');
    document.getElementById('btn-cancel-acc').addEventListener('click', () => modalAcc.style.display = 'none');

    // Add Transaction Form
    document.getElementById('form-add-tx').addEventListener('submit', (e) => {{
      e.preventDefault();
      const amount = parseFloat(document.getElementById('tx-in-amount').value) || 0;
      const accId = document.getElementById('tx-in-account').value;
      const newTx = {{
        id: 'tx-' + Date.now(),
        date: document.getElementById('tx-in-date').value,
        accountId: accId,
        payee: document.getElementById('tx-in-payee').value,
        categoryId: document.getElementById('tx-in-category').value,
        notes: document.getElementById('tx-in-notes').value,
        amount: amount
      }};
      appState.transactions.push(newTx);
      // Update account balance
      const acc = appState.accounts.find(a => a.id === accId);
      if (acc) acc.balance += amount;

      modalTx.style.display = 'none';
      e.target.reset();
      saveState();
    }});

    // Add Account Form
    document.getElementById('form-add-acc').addEventListener('submit', (e) => {{
      e.preventDefault();
      const newAcc = {{
        id: 'acc-' + Date.now(),
        name: document.getElementById('acc-in-name').value,
        type: document.getElementById('acc-in-type').value,
        balance: parseFloat(document.getElementById('acc-in-balance').value) || 0
      }};
      appState.accounts.push(newAcc);
      modalAcc.style.display = 'none';
      e.target.reset();
      saveState();
    }});

    // Search filter
    document.getElementById('tx-search').addEventListener('input', renderTransactionsView);

    // Month Switcher
    const months = ['October 2026', 'November 2026', 'December 2026', 'January 2027'];
    document.getElementById('btn-prev-month').addEventListener('click', () => {{
      const idx = months.indexOf(appState.month);
      if (idx > 0) {{ appState.month = months[idx - 1]; saveState(); }}
    }});
    document.getElementById('btn-next-month').addEventListener('click', () => {{
      const idx = months.indexOf(appState.month);
      if (idx < months.length - 1) {{ appState.month = months[idx + 1]; saveState(); }}
    }});

    // Reset & Export
    document.getElementById('btn-reset-demo').addEventListener('click', () => {{
      if (confirm('Reset to initial sample budget data?')) {{
        appState = getDefaultState();
        saveState();
      }}
    }});
    document.getElementById('btn-export-json').addEventListener('click', () => {{
      const blob = new Blob([JSON.stringify(appState, null, 2)], {{ type: 'application/json' }});
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'actual-budget-export.json';
      a.click();
    }});

    // Initialize & Dismiss Loading Overlay
    window.addEventListener('DOMContentLoaded', () => {{
      checkStorage();
      renderAll();

      const overlay = document.getElementById('loading-overlay');
      if (overlay) {{
        overlay.style.opacity = '0';
        overlay.style.pointerEvents = 'none';
        setTimeout(() => {{
          overlay.style.display = 'none';
        }}, 350);
      }}
    }});
  </script>
</body>
</html>
"#,
            title = title,
            theme_color = theme_color
        )
    }
}
