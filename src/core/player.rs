use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use crate::utils::{GameError, GameResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub health: i32,
    pub max_health: i32,
    pub experience: i32,
    pub level: i32,
    pub strength: i32,
    pub intelligence: i32,
    pub charisma: i32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            health: 100,
            max_health: 100,
            experience: 0,
            level: 1,
            strength: 10,
            intelligence: 10,
            charisma: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub item_type: ItemType,
    pub quantity: i32,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemType {
    Weapon,
    Armor,
    Consumable,
    KeyItem,
    Treasure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    pub stats: PlayerStats,
    pub inventory: Vec<InventoryItem>,
}

impl Player {
    pub fn new<S: Into<String>>(name: S, initial_stats: Option<PlayerStats>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            stats: initial_stats.unwrap_or_default(),
            inventory: Vec::new(),
        }
    }

    pub fn modify_stat(&mut self, stat_name: &str, value: i32, operation: StatOperation) -> GameResult<()> {
        match stat_name {
            "health" => {
                let new_value = self.apply_operation(self.stats.health, value, operation);
                self.stats.health = new_value.max(0).min(self.stats.max_health);
            }
            "max_health" => {
                let new_value = self.apply_operation(self.stats.max_health, value, operation);
                self.stats.max_health = new_value.max(1);
                if self.stats.health > self.stats.max_health {
                    self.stats.health = self.stats.max_health;
                }
            }
            "experience" => {
                let old_level = self.stats.level;
                let new_value = self.apply_operation(self.stats.experience, value, operation);
                self.stats.experience = new_value.max(0);
                self.update_level();
                
                if self.stats.level > old_level {
                    self.level_up_benefits(self.stats.level - old_level);
                }
            }
            "strength" => {
                let new_value = self.apply_operation(self.stats.strength, value, operation);
                self.stats.strength = new_value.max(1);
            }
            "intelligence" => {
                let new_value = self.apply_operation(self.stats.intelligence, value, operation);
                self.stats.intelligence = new_value.max(1);
            }
            "charisma" => {
                let new_value = self.apply_operation(self.stats.charisma, value, operation);
                self.stats.charisma = new_value.max(1);
            }
            _ => return Err(GameError::player(format!("Unknown stat: {}", stat_name))),
        }
        Ok(())
    }

    pub fn add_item(&mut self, item: InventoryItem) {
        if let Some(existing) = self.inventory.iter_mut().find(|i| i.id == item.id) {
            existing.quantity += item.quantity;
        } else {
            self.inventory.push(item);
        }
    }

    pub fn remove_item(&mut self, item_id: &str, quantity: i32) -> GameResult<()> {
        if let Some(pos) = self.inventory.iter().position(|i| i.id == item_id) {
            let item = &mut self.inventory[pos];
            if item.quantity >= quantity {
                item.quantity -= quantity;
                if item.quantity <= 0 {
                    self.inventory.remove(pos);
                }
                Ok(())
            } else {
                Err(GameError::player(format!("Not enough items: {} (have: {}, need: {})", 
                    item_id, item.quantity, quantity)))
            }
        } else {
            Err(GameError::player(format!("Item not found: {}", item_id)))
        }
    }

    pub fn has_item(&self, item_id: &str, quantity: i32) -> bool {
        self.inventory
            .iter()
            .find(|i| i.id == item_id)
            .map_or(false, |item| item.quantity >= quantity)
    }

    pub fn get_item(&self, item_id: &str) -> Option<&InventoryItem> {
        self.inventory.iter().find(|i| i.id == item_id)
    }

    pub fn use_consumable(&mut self, item_id: &str) -> GameResult<()> {
        let item = self.get_item(item_id)
            .ok_or_else(|| GameError::player(format!("Item not found: {}", item_id)))?;
        
        if !matches!(item.item_type, ItemType::Consumable) {
            return Err(GameError::player("Item is not consumable".to_string()));
        }

        // Apply consumable effects based on properties
        let item_properties = item.properties.clone();
        self.remove_item(item_id, 1)?;

        // Apply effects
        if let Some(health_restore) = item_properties.get("health_restore") {
            if let Some(value) = health_restore.as_i64() {
                self.modify_stat("health", value as i32, StatOperation::Add)?;
            }
        }

        if let Some(strength_boost) = item_properties.get("strength_boost") {
            if let Some(value) = strength_boost.as_i64() {
                self.modify_stat("strength", value as i32, StatOperation::Add)?;
            }
        }

        if let Some(intelligence_boost) = item_properties.get("intelligence_boost") {
            if let Some(value) = intelligence_boost.as_i64() {
                self.modify_stat("intelligence", value as i32, StatOperation::Add)?;
            }
        }

        if let Some(charisma_boost) = item_properties.get("charisma_boost") {
            if let Some(value) = charisma_boost.as_i64() {
                self.modify_stat("charisma", value as i32, StatOperation::Add)?;
            }
        }

        Ok(())
    }

    pub fn is_alive(&self) -> bool {
        self.stats.health > 0
    }

    pub fn get_level(&self) -> i32 {
        self.stats.level
    }

    pub fn experience_to_next_level(&self) -> i32 {
        let next_level_exp = self.experience_required_for_level(self.stats.level + 1);
        next_level_exp - self.stats.experience
    }

    pub fn get_inventory_by_type(&self, item_type: ItemType) -> Vec<&InventoryItem> {
        self.inventory
            .iter()
            .filter(|item| std::mem::discriminant(&item.item_type) == std::mem::discriminant(&item_type))
            .collect()
    }

    pub fn get_total_inventory_weight(&self) -> i32 {
        self.inventory
            .iter()
            .map(|item| {
                let weight = item
                    .properties
                    .get("weight")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;
                weight * item.quantity
            })
            .sum()
    }

    pub fn get_inventory_value(&self) -> i32 {
        self.inventory
            .iter()
            .map(|item| {
                let value = item
                    .properties
                    .get("value")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                value * item.quantity
            })
            .sum()
    }

    fn apply_operation(&self, current: i32, value: i32, operation: StatOperation) -> i32 {
        match operation {
            StatOperation::Set => value,
            StatOperation::Add => current + value,
            StatOperation::Subtract => current - value,
            StatOperation::Multiply => current * value,
        }
    }

    fn update_level(&mut self) {
        let new_level = self.calculate_level_from_experience(self.stats.experience);
        self.stats.level = new_level;
    }

    fn calculate_level_from_experience(&self, experience: i32) -> i32 {
        // Level = floor(sqrt(experience / 100)) + 1
        ((experience as f32 / 100.0).sqrt().floor() as i32) + 1
    }

    fn experience_required_for_level(&self, level: i32) -> i32 {
        // Inverse of level calculation: exp = (level - 1)² * 100
        (level - 1).pow(2) * 100
    }

    fn level_up_benefits(&mut self, levels_gained: i32) {
        // Grant stat increases on level up
        self.stats.max_health += levels_gained * 10;
        self.stats.health = self.stats.max_health; // Full heal on level up
        self.stats.strength += levels_gained;
        self.stats.intelligence += levels_gained;
        self.stats.charisma += levels_gained;
    }
}

#[derive(Debug, Clone)]
pub enum StatOperation {
    Set,
    Add,
    Subtract,
    Multiply,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_creation() {
        let player = Player::new("Test Player", None);
        assert_eq!(player.name, "Test Player");
        assert_eq!(player.stats.health, 100);
        assert_eq!(player.stats.level, 1);
    }

    #[test]
    fn test_stat_modification() {
        let mut player = Player::new("Test", None);
        player.modify_stat("health", -20, StatOperation::Add).unwrap();
        assert_eq!(player.stats.health, 80);
        
        player.modify_stat("strength", 5, StatOperation::Add).unwrap();
        assert_eq!(player.stats.strength, 15);
    }

    #[test]
    fn test_inventory_management() {
        let mut player = Player::new("Test", None);
        
        let item = InventoryItem {
            id: "sword".to_string(),
            name: "Iron Sword".to_string(),
            description: "A sturdy iron sword".to_string(),
            item_type: ItemType::Weapon,
            quantity: 1,
            properties: HashMap::new(),
        };
        
        player.add_item(item);
        assert!(player.has_item("sword", 1));
        assert_eq!(player.inventory.len(), 1);
        
        player.remove_item("sword", 1).unwrap();
        assert!(!player.has_item("sword", 1));
        assert_eq!(player.inventory.len(), 0);
    }

    #[test]
    fn test_experience_and_leveling() {
        let mut player = Player::new("Test", None);
        
        // Test experience gain and leveling
        player.modify_stat("experience", 100, StatOperation::Add).unwrap();
        assert_eq!(player.stats.level, 2);
        assert_eq!(player.stats.max_health, 110); // +10 from level up
        
        player.modify_stat("experience", 300, StatOperation::Add).unwrap();
        assert_eq!(player.stats.level, 3);
    }
}