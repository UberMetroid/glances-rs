//! Swap memory plugin — total/used/free/percent.

use std::collections::BTreeMap;

use crate::core::error::Result;
use crate::platform as plat;
use crate::core::events::EventLog;
use crate::core::plugin::{GlancesPluginModel, Plugin};
use crate::core::value::Value;

pub const NAME: &str = "memswap";

/// Fraction of `MemTotal` that must remain available before swap occupancy is
/// treated as a real symptom rather than inert ballast.
///
/// Swap is a symptom of memory pressure, not a problem in itself. A kernel
/// holding cold pages in a full zram device while gigabytes of RAM sit free
/// costs nothing and resolves itself the instant memory is actually needed.
/// Alerting on swap ratio alone latches CRITICAL permanently in that state,
/// which trains operators to ignore the very alerts meant to be read.
const MEM_COMFORT_RATIO: f64 = 0.15;

pub fn register(stats: &crate::core::stats::GlancesStats) {
    stats.register(Box::new(MemswapPlugin::new()));
}

pub struct MemswapPlugin {
    base: GlancesPluginModel,
    /// `MemAvailable / MemTotal`, taken from the same `/proc/meminfo` read as
    /// the swap figures. Held outside the stats payload deliberately: the
    /// JSON shape is a frozen wire contract, so this stays a private field.
    mem_free_ratio: f64,
}

impl Default for MemswapPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MemswapPlugin {
    pub fn new() -> Self {
        let mut m = BTreeMap::new();
        for k in &["total", "used", "free", "percent"] {
            m.insert(k.to_string(), Value::Float(0.0));
        }
        Self {
            base: GlancesPluginModel::new(NAME, Value::Object(m)),
            // No reading yet, so claim nothing about pressure. The first
            // update overwrites this before any view is built.
            mem_free_ratio: 1.0,
        }
    }

    /// True when memory is constrained enough that swap state matters.
    fn memory_constrained(&self) -> bool {
        self.mem_free_ratio <= MEM_COMFORT_RATIO
    }
}

impl Plugin for MemswapPlugin {
    fn name(&self) -> &'static str { NAME }
    fn reset(&mut self) { self.base.reset(); }
    fn stats(&self) -> &Value { &self.base.stats }
    fn model(&self) -> Option<&GlancesPluginModel> { Some(&self.base) }
    fn model_mut(&mut self) -> Option<&mut GlancesPluginModel> { Some(&mut self.base) }
    fn stats_mut(&mut self) -> &mut Value { &mut self.base.stats }
    fn history_items(&self) -> &[&'static str] { &["percent"] }
    fn update(&mut self) -> Result<()> {
        let info = plat::linux::proc_meminfo::read()?;
        let used = info.swap_total.saturating_sub(info.swap_free);
        let pct = if info.swap_total > 0 { (used as f64 / info.swap_total as f64) * 100.0 } else { 0.0 };
        self.mem_free_ratio = if info.total > 0 {
            (info.available as f64 / info.total as f64).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if let Some(obj) = self.base.stats.as_object_mut() {
            obj.insert("total".into(), Value::Float(info.swap_total as f64));
            obj.insert("used".into(), Value::Float(used as f64));
            obj.insert("free".into(), Value::Float(info.swap_free as f64));
            obj.insert("percent".into(), Value::Float(pct));
        }
        Ok(())
    }
    fn update_views(&mut self, events: &mut EventLog) {
        // Read the pressure flag before borrowing `self` mutably below.
        let constrained = self.memory_constrained();
        if let Some(m) = self.model_mut() {
            m.build_views(&[], None, None);
            let (used, total) = match m.stats.as_object() {
                Some(o) => (
                    o.get("used").and_then(Value::as_f64).unwrap_or(0.0),
                    o.get("total").and_then(Value::as_f64).unwrap_or(0.0),
                ),
                None => return,
            };
            let state = if total > 0.0 && constrained {
                m.get_alert_log(used, total, "", Some(&mut *events))
            } else {
                // Comfortable memory: swap occupancy is inert. Still publish
                // the field so the view shape is unchanged, just untriggered.
                "DEFAULT".to_string()
            };
            m.views.entry(String::new()).or_default().insert("percent".into(), state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive a plugin into a known swap/memory state without touching the
    /// real `/proc/meminfo`, then report the `percent` view state.
    ///
    /// Applies the same default limits the live pipeline applies
    /// (`stats.rs` -> `default_limit_entries`), otherwise a bare model has no
    /// thresholds and every state reads DEFAULT.
    fn percent_state(swap_used: f64, swap_total: f64, mem_free_ratio: f64) -> String {
        let mut p = MemswapPlugin::new();
        p.base
            .apply_default_limits(&crate::core::alerts::default_limit_entries(NAME), 1);
        if let Some(o) = p.base.stats.as_object_mut() {
            o.insert("used".into(), Value::Float(swap_used));
            o.insert("total".into(), Value::Float(swap_total));
        }
        p.mem_free_ratio = mem_free_ratio;
        let mut events = EventLog::new(16);
        p.update_views(&mut events);
        p.base
            .views
            .get("")
            .and_then(|d| d.get("percent"))
            .cloned()
            .unwrap_or_default()
    }

    #[test]
    fn full_swap_with_comfortable_memory_does_not_alert() {
        // The regression this guards: zram at 100% with 76% of RAM still
        // available must not raise a permanent CRITICAL.
        let s = percent_state(8192.0, 8192.0, 0.76);
        assert_eq!(s, "DEFAULT", "comfortable memory must suppress the swap alert, got {s}");
    }

    #[test]
    fn full_swap_with_constrained_memory_still_alerts() {
        // 8% available is genuine pressure — the alert must survive.
        let s = percent_state(8192.0, 8192.0, 0.08);
        assert_ne!(s, "DEFAULT", "constrained memory must still report a swap alert, got {s}");
    }

    #[test]
    fn comfort_boundary_is_inclusive() {
        assert!(!MemswapPlugin { base: GlancesPluginModel::new(NAME, Value::Null), mem_free_ratio: 0.1501 }.memory_constrained());
        assert!(MemswapPlugin { base: GlancesPluginModel::new(NAME, Value::Null), mem_free_ratio: 0.15 }.memory_constrained());
        assert!(MemswapPlugin { base: GlancesPluginModel::new(NAME, Value::Null), mem_free_ratio: 0.0 }.memory_constrained());
    }

    #[test]
    fn unused_swap_with_constrained_memory_stays_quiet() {
        // Low swap + low memory is not a swap problem; it reads OK, not a
        // triggered band. The `_LOG` suffix is the logging view's marker.
        assert_eq!(percent_state(100.0, 8192.0, 0.08), "OK_LOG");
    }
}
