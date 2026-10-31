use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Cut,
    Keep,
    Speed(f64),
}

impl Action {
    pub fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim().to_lowercase();
        if trimmed == "cut" {
            Ok(Action::Cut)
        } else if trimmed == "keep" || trimmed == "nil" || trimmed == "audio" {
            Ok(Action::Keep)
        } else if let Some(speed_str) = trimmed.strip_prefix("speed:") {
            let speed = speed_str
                .parse::<f64>()
                .map_err(|e| format!("Invalid speed '{}': {}", speed_str, e))?;
            Ok(Action::Speed(speed))
        } else if let Ok(speed) = trimmed.parse::<f64>() {
            Ok(Action::Speed(speed))
        } else {
            Err(format!("Unknown action '{}'", s))
        }
    }

    pub fn speed_multiplier(&self) -> f64 {
        match self {
            Action::Cut => 99999.0, // Sentinel for cutting out
            Action::Keep => 1.0,
            Action::Speed(s) => *s,
        }
    }

    pub fn is_cut(&self) -> bool {
        matches!(self, Action::Cut) || (self.speed_multiplier() > 1000.0)
    }
}

#[derive(Debug, Clone)]
pub struct LabelManager {
    actions: HashMap<u8, Action>,
}

impl LabelManager {
    pub fn new() -> Self {
        let mut actions = HashMap::new();
        actions.insert(0, Action::Cut); // Label 0 is cut by default
        actions.insert(1, Action::Keep); // Label 1 is keep by default
        Self { actions }
    }

    pub fn set_action(&mut self, label: u8, action: Action) {
        self.actions.insert(label, action);
    }

    pub fn get_action(&self, label: u8) -> Action {
        self.actions.get(&label).copied().unwrap_or(Action::Keep)
    }

    /// Resolve an array of layer labels (where higher label number has higher priority)
    pub fn resolve_layers(&self, frame_labels: &[u8]) -> Vec<Action> {
        frame_labels
            .iter()
            .map(|&label| self.get_action(label))
            .collect()
    }
}

impl Default for LabelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_parsing() {
        assert_eq!(Action::parse("cut").unwrap(), Action::Cut);
        assert_eq!(Action::parse("keep").unwrap(), Action::Keep);
        assert_eq!(Action::parse("speed:2.0").unwrap(), Action::Speed(2.0));
        assert_eq!(Action::parse("1.5").unwrap(), Action::Speed(1.5));
    }

    #[test]
    fn test_label_resolution() {
        let mut lm = LabelManager::new();
        lm.set_action(2, Action::Speed(1.5));

        assert_eq!(lm.get_action(0), Action::Cut);
        assert_eq!(lm.get_action(1), Action::Keep);
        assert_eq!(lm.get_action(2), Action::Speed(1.5));
    }
}
