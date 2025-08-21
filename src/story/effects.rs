use serde::{Deserialize, Serialize};
use crate::core::InventoryItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub effect_type: EffectType,
    pub key: String,
    pub value: serde_json::Value,
    pub operation: Option<EffectOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectType {
    SetFlag,
    ModifyStat,
    AddItem,
    RemoveItem,
    ModifyHealth,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectOperation {
    Set,
    Add,
    Subtract,
    Multiply,
}

impl Effect {
    pub fn new(
        effect_type: EffectType,
        key: String,
        value: serde_json::Value,
        operation: Option<EffectOperation>,
    ) -> Self {
        Self {
            effect_type,
            key,
            value,
            operation,
        }
    }

    // Convenience constructors
    pub fn set_flag<S: Into<String>>(key: S, value: bool) -> Self {
        Self::new(
            EffectType::SetFlag,
            key.into(),
            serde_json::Value::Bool(value),
            None,
        )
    }

    pub fn modify_stat<S: Into<String>>(key: S, value: i32, operation: EffectOperation) -> Self {
        Self::new(
            EffectType::ModifyStat,
            key.into(),
            serde_json::Value::Number(serde_json::Number::from(value)),
            Some(operation),
        )
    }

    pub fn add_health(value: i32) -> Self {
        Self::new(
            EffectType::ModifyHealth,
            "health".to_string(),
            serde_json::Value::Number(serde_json::Number::from(value)),
            Some(EffectOperation::Add),
        )
    }

    pub fn subtract_health(value: i32) -> Self {
        Self::new(
            EffectType::ModifyHealth,
            "health".to_string(),
            serde_json::Value::Number(serde_json::Number::from(value)),
            Some(EffectOperation::Subtract),
        )
    }

    pub fn add_experience(value: i32) -> Self {
        Self::modify_stat("experience", value, EffectOperation::Add)
    }

    pub fn add_item_effect(item: InventoryItem, quantity: Option<i32>) -> Self {
        let mut item_data = serde_json::to_value(item).unwrap();
        if let Some(qty) = quantity {
            if let Some(obj) = item_data.as_object_mut() {
                obj.insert("quantity".to_string(), serde_json::Value::Number(serde_json::Number::from(qty)));
            }
        }

        Self::new(
            EffectType::AddItem,
            "item".to_string(),
            item_data,
            None,
        )
    }

    pub fn remove_item_effect<S: Into<String>>(item_id: S, quantity: i32) -> Self {
        let remove_data = serde_json::json!({
            "id": item_id.into(),
            "quantity": quantity
        });

        Self::new(
            EffectType::RemoveItem,
            item_id.into(),
            remove_data,
            None,
        )
    }

    pub fn custom<S: Into<String>>(key: S, value: serde_json::Value, operation: Option<EffectOperation>) -> Self {
        Self::new(EffectType::Custom, key.into(), value, operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ItemType, PlayerStats};
    use std::collections::HashMap;

    #[test]
    fn test_effect_creation() {
        let effect = Effect::set_flag("test_flag", true);
        assert!(matches!(effect.effect_type, EffectType::SetFlag));
        assert_eq!(effect.key, "test_flag");
        assert_eq!(effect.value, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_stat_effect() {
        let effect = Effect::modify_stat("strength", 5, EffectOperation::Add);
        assert!(matches!(effect.effect_type, EffectType::ModifyStat));
        assert_eq!(effect.key, "strength");
        assert!(matches!(effect.operation, Some(EffectOperation::Add)));
    }

    #[test]
    fn test_health_effects() {
        let heal_effect = Effect::add_health(25);
        assert!(matches!(heal_effect.effect_type, EffectType::ModifyHealth));
        assert!(matches!(heal_effect.operation, Some(EffectOperation::Add)));

        let damage_effect = Effect::subtract_health(10);
        assert!(matches!(damage_effect.effect_type, EffectType::ModifyHealth));
        assert!(matches!(damage_effect.operation, Some(EffectOperation::Subtract)));
    }

    #[test]
    fn test_item_effects() {
        let item = InventoryItem {
            id: "potion".to_string(),
            name: "Health Potion".to_string(),
            description: "Restores health".to_string(),
            item_type: ItemType::Consumable,
            quantity: 1,
            properties: HashMap::new(),
        };

        let effect = Effect::add_item_effect(item, Some(3));
        assert!(matches!(effect.effect_type, EffectType::AddItem));

        let remove_effect = Effect::remove_item_effect("sword", 1);
        assert!(matches!(remove_effect.effect_type, EffectType::RemoveItem));
    }
}