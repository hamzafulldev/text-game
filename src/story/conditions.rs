use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: ConditionType,
    pub key: String,
    pub operator: ComparisonOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    Flag,
    Stat,
    Inventory,
    SceneVisited,
    Level,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    Has,
    NotHas,
    Contains,
    NotContains,
}

impl Condition {
    pub fn new(
        condition_type: ConditionType,
        key: String,
        operator: ComparisonOperator,
        value: serde_json::Value,
    ) -> Self {
        Self {
            condition_type,
            key,
            operator,
            value,
        }
    }

    // Convenience constructors
    pub fn flag_equals<S: Into<String>>(key: S, value: bool) -> Self {
        Self::new(
            ConditionType::Flag,
            key.into(),
            ComparisonOperator::Equals,
            serde_json::Value::Bool(value),
        )
    }

    pub fn stat_greater_than<S: Into<String>>(key: S, value: i32) -> Self {
        Self::new(
            ConditionType::Stat,
            key.into(),
            ComparisonOperator::GreaterThan,
            serde_json::Value::Number(serde_json::Number::from(value)),
        )
    }

    pub fn stat_greater_equal<S: Into<String>>(key: S, value: i32) -> Self {
        Self::new(
            ConditionType::Stat,
            key.into(),
            ComparisonOperator::GreaterEqual,
            serde_json::Value::Number(serde_json::Number::from(value)),
        )
    }

    pub fn has_item<S: Into<String>>(key: S, quantity: i32) -> Self {
        Self::new(
            ConditionType::Inventory,
            key.into(),
            ComparisonOperator::GreaterEqual,
            serde_json::Value::Number(serde_json::Number::from(quantity)),
        )
    }

    pub fn scene_visited<S: Into<String>>(scene_id: S) -> Self {
        Self::new(
            ConditionType::SceneVisited,
            scene_id.into(),
            ComparisonOperator::Equals,
            serde_json::Value::Bool(true),
        )
    }

    pub fn level_at_least(level: i32) -> Self {
        Self::new(
            ConditionType::Level,
            "level".to_string(),
            ComparisonOperator::GreaterEqual,
            serde_json::Value::Number(serde_json::Number::from(level)),
        )
    }

    pub fn custom<S: Into<String>>(key: S, operator: ComparisonOperator, value: serde_json::Value) -> Self {
        Self::new(ConditionType::Custom, key.into(), operator, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_creation() {
        let condition = Condition::flag_equals("test_flag", true);
        assert!(matches!(condition.condition_type, ConditionType::Flag));
        assert_eq!(condition.key, "test_flag");
        assert!(matches!(condition.operator, ComparisonOperator::Equals));
    }

    #[test]
    fn test_stat_condition() {
        let condition = Condition::stat_greater_than("strength", 15);
        assert!(matches!(condition.condition_type, ConditionType::Stat));
        assert_eq!(condition.key, "strength");
        assert!(matches!(condition.operator, ComparisonOperator::GreaterThan));
    }

    #[test]
    fn test_inventory_condition() {
        let condition = Condition::has_item("sword", 1);
        assert!(matches!(condition.condition_type, ConditionType::Inventory));
        assert_eq!(condition.key, "sword");
        assert!(matches!(condition.operator, ComparisonOperator::GreaterEqual));
    }
}