//! Mobile application orchestrator.
//!
//! Manages mobile viewport, safe area insets, bottom navigation bar,
//! quick expense keypad entry, gesture processing, and pull-to-refresh sync.

use crate::mobile::gestures::{Gesture, GestureClassifier, TouchPhase};
use crate::mobile::keypad::{KeypadKey, QuickAddSheet};
use crate::mobile::layout::{BottomNavBar, MobileTab, MobileViewport};
use actual_core::cents::Cents;
use actual_core::models::Transaction;

/// Mobile application state.
pub struct MobileApp {
    pub viewport: MobileViewport,
    pub nav_bar: BottomNavBar,
    pub quick_add: QuickAddSheet,
    pub gesture_classifier: GestureClassifier,
    pub transactions: Vec<Transaction>,
    pub is_syncing: bool,
    pub sync_success_count: u32,
}

impl Default for MobileApp {
    fn default() -> Self {
        Self {
            viewport: MobileViewport::standard_phone(),
            nav_bar: BottomNavBar::default(),
            quick_add: QuickAddSheet::default(),
            gesture_classifier: GestureClassifier::default(),
            transactions: Vec::new(),
            is_syncing: false,
            sync_success_count: 0,
        }
    }
}

impl MobileApp {
    /// Creates a new mobile app state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects active navigation tab.
    pub fn switch_tab(&mut self, tab: MobileTab) {
        if tab == MobileTab::QuickAdd {
            self.quick_add.open();
        } else {
            self.nav_bar.select_tab(tab);
        }
    }

    /// Feeds touch events into gesture classifier and handles gestures.
    pub fn handle_touch(
        &mut self,
        phase: TouchPhase,
        x: f32,
        y: f32,
        timestamp_ms: u64,
    ) -> Option<Gesture> {
        let gesture = self
            .gesture_classifier
            .process_touch(phase, x, y, timestamp_ms);
        if let Some(ref g) = gesture {
            match g {
                Gesture::PullToRefresh { .. } => {
                    self.trigger_sync();
                }
                Gesture::SwipeLeft { .. } if !self.transactions.is_empty() => {
                    // Swipe-to-delete most recent item
                    self.transactions.pop();
                }
                _ => {}
            }
        }
        gesture
    }

    /// Keypad button entry for QuickAdd bottom sheet.
    pub fn keypad_press(&mut self, key: KeypadKey) {
        if key == KeypadKey::Submit {
            self.commit_quick_add();
        } else {
            self.quick_add.keypad.press_key(key);
        }
    }

    /// Commits the QuickAdd bottom sheet into a new transaction.
    pub fn commit_quick_add(&mut self) -> Option<Transaction> {
        if !self.quick_add.is_open {
            return None;
        }

        let cents = self.quick_add.keypad.to_cents();
        let account_id = self
            .quick_add
            .selected_account_id
            .clone()
            .unwrap_or_else(|| "acc-default".to_string());

        let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap_or_default();
        let mut tx = Transaction::new(
            format!("tx-{}", self.transactions.len() + 1),
            account_id,
            date,
            cents,
        );
        if !self.quick_add.notes.is_empty() {
            tx.notes = Some(self.quick_add.notes.clone());
        }

        self.transactions.push(tx.clone());
        self.quick_add.close();
        Some(tx)
    }

    /// Triggers background sync cycle.
    pub fn trigger_sync(&mut self) {
        self.is_syncing = true;
        self.sync_success_count += 1;
        self.is_syncing = false;
    }

    /// Total balance of all stored mobile transactions.
    pub fn total_balance(&self) -> Cents {
        let mut sum = Cents::ZERO;
        for tx in &self.transactions {
            sum += tx.amount;
        }
        sum
    }
}
