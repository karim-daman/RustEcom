#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{Row, sqlite::SqliteRow};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for User {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(User {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            full_name: row.try_get("full_name")?,
            phone: row.try_get("phone")?,
            is_active: row.try_get::<i64, _>("is_active")? != 0,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub region: Option<String>,
    pub postal_code: String,
    pub country: String,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for Address {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Address {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            label: row.try_get("label")?,
            line1: row.try_get("line1")?,
            line2: row.try_get("line2")?,
            city: row.try_get("city")?,
            region: row.try_get("region")?,
            postal_code: row.try_get("postal_code")?,
            country: row.try_get("country")?,
            is_default: row.try_get::<i64, _>("is_default")? != 0,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for Category {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Category {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            slug: row.try_get("slug")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discount {
    pub id: Uuid,
    pub name: String,
    pub percent: i64,
    pub category_id: Option<Uuid>,
    pub min_price_cents: Option<i64>,
    pub max_price_cents: Option<i64>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for Discount {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Discount {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            name: row.try_get("name")?,
            percent: row.try_get("percent")?,
            category_id: row.try_get::<Option<String>, _>("category_id")?.and_then(|s| Uuid::parse_str(&s).ok()),
            min_price_cents: row.try_get("min_price_cents")?,
            max_price_cents: row.try_get("max_price_cents")?,
            active: row.try_get::<i64, _>("active")? != 0,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Uuid,
    pub sku: String,
    pub title: String,
    pub description: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub active: bool,
    pub images: Json<Vec<String>>,
    pub created_at: String,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for Product {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Product {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            sku: row.try_get("sku")?,
            title: row.try_get("title")?,
            description: row.try_get("description")?,
            price_cents: row.try_get("price_cents")?,
            currency: row.try_get("currency")?,
            active: row.try_get::<i64, _>("active")? != 0,
            images: row.try_get("images")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCategory {
    pub product_id: Uuid,
    pub category_id: Uuid,
}

impl sqlx::FromRow<'_, SqliteRow> for ProductCategory {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(ProductCategory {
            product_id: Uuid::parse_str(&row.try_get::<String, _>("product_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            category_id: Uuid::parse_str(&row.try_get::<String, _>("category_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub product_id: Uuid,
    pub quantity: i64,
    pub reserved: i64,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for Inventory {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Inventory {
            product_id: Uuid::parse_str(&row.try_get::<String, _>("product_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            quantity: row.try_get("quantity")?,
            reserved: row.try_get("reserved")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cart {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for Cart {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Cart {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    pub cart_id: Uuid,
    pub product_id: Uuid,
    pub quantity: i64,
    pub added_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for CartItem {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(CartItem {
            cart_id: Uuid::parse_str(&row.try_get::<String, _>("cart_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            product_id: Uuid::parse_str(&row.try_get::<String, _>("product_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            quantity: row.try_get("quantity")?,
            added_at: row.try_get("added_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub shipping_address_id: Option<Uuid>,
    pub billing_address_id: Option<Uuid>,
    pub status: String,
    pub total_cents: i64,
    pub currency: String,
    pub placed_at: String,
    pub fulfilled_at: Option<String>,
    pub canceled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for Order {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Order {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            user_id: Uuid::parse_str(&row.try_get::<String, _>("user_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            shipping_address_id: row.try_get::<Option<String>, _>("shipping_address_id")?.and_then(|s| Uuid::parse_str(&s).ok()),
            billing_address_id: row.try_get::<Option<String>, _>("billing_address_id")?.and_then(|s| Uuid::parse_str(&s).ok()),
            status: row.try_get("status")?,
            total_cents: row.try_get("total_cents")?,
            currency: row.try_get("currency")?,
            placed_at: row.try_get("placed_at")?,
            fulfilled_at: row.try_get("fulfilled_at")?,
            canceled_at: row.try_get("canceled_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub quantity: i64,
    pub unit_price_cents: i64,
    pub subtotal_cents: i64,
}

impl sqlx::FromRow<'_, SqliteRow> for OrderItem {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(OrderItem {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            order_id: Uuid::parse_str(&row.try_get::<String, _>("order_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            product_id: Uuid::parse_str(&row.try_get::<String, _>("product_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            quantity: row.try_get("quantity")?,
            unit_price_cents: row.try_get("unit_price_cents")?,
            subtotal_cents: row.try_get("subtotal_cents")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub order_id: Uuid,
    pub method: String,
    pub provider: Option<String>,
    pub provider_transaction_id: Option<String>,
    pub amount_cents: i64,
    pub status: String,
    pub paid_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl sqlx::FromRow<'_, SqliteRow> for Payment {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Payment {
            id: Uuid::parse_str(&row.try_get::<String, _>("id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            order_id: Uuid::parse_str(&row.try_get::<String, _>("order_id")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            method: row.try_get("method")?,
            provider: row.try_get("provider")?,
            provider_transaction_id: row.try_get("provider_transaction_id")?,
            amount_cents: row.try_get("amount_cents")?,
            status: row.try_get("status")?,
            paid_at: row.try_get("paid_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}
