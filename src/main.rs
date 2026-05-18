mod schema;

use anyhow::Context;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Row, SqlitePool, ValueRef, QueryBuilder};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use uuid::Uuid;
use chrono::Local;
use axum::middleware::Next;
use axum::body::Body;
use axum::http::{header::{ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN}, HeaderValue, Method, Request};

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    log_path: Arc<PathBuf>,
}

type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "error": self.message,
        }));
        (self.status, body).into_response()
    }
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

#[derive(Serialize)]
struct PublicUser {
    id: Uuid,
    email: String,
    full_name: Option<String>,
    is_active: bool,
}

#[derive(Debug, Serialize, FromRow)]
struct CartItemResponse {
    product_id: String,
    title: String,
    quantity: i64,
    unit_price_cents: i64,
    subtotal_cents: i64,
}

#[derive(Debug, Serialize, FromRow)]
struct OrderSummary {
    id: String,
    status: String,
    total_cents: i64,
    currency: String,
    placed_at: String,
}

#[derive(Serialize)]
struct CartResponse {
    cart_id: Uuid,
    items: Vec<CartItemResponse>,
    total_cents: i64,
}

#[derive(Deserialize)]
struct AddToCartRequest {
    user_id: Uuid,
    product_id: Uuid,
    quantity: i64,
}

#[derive(Deserialize)]
struct CheckoutRequest {
    user_id: Uuid,
    shipping_address_id: Option<Uuid>,
    billing_address_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ListProductsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    category_id: Option<String>,
    #[serde(default)]
    min_price: Option<i64>,
    #[serde(default)]
    max_price: Option<i64>,
    #[serde(default)]
    search: Option<String>,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize)]
struct PaginatedProducts {
    items: Vec<schema::Product>,
    total: i64,
    limit: i64,
    offset: i64,
    has_more: bool,
}

async fn log_event(log_path: &Arc<PathBuf>, event: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_message = format!("[{}] {}\n", timestamp, event);
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path.as_path())
        .await
    {
        let _ = file.write_all(log_message.as_bytes()).await;
        let _ = file.sync_all().await;
    }
}

async fn logging_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    let event = format!("{} {}", method, uri);
    log_event(&state.log_path, &event).await;
    
    next.run(req).await
}

async fn cors_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    if req.method() == Method::OPTIONS {
        let mut response = Response::new(Body::empty());
        let headers = response.headers_mut();
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
        headers.insert(ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"));
        headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("*"));
        return response;
    }

    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"));
    headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("*"));
    response
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn style() -> impl IntoResponse {
    ([("content-type", "text/css; charset=utf-8")], include_str!("../static/style.css"))
}

async fn app_js() -> impl IntoResponse {
    ([("content-type", "application/javascript; charset=utf-8")], include_str!("../static/app.js"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url_env = std::env::var("DATABASE_URL").unwrap_or_else(|_| "ecommerce.db".to_string());
    let database_path = if database_url_env.starts_with("sqlite://") {
        let file_part = &database_url_env["sqlite://".len()..];
        if file_part.starts_with('/') {
            PathBuf::from(file_part)
        } else {
            std::env::current_dir()
                .context("Failed to resolve current working directory")?
                .join(file_part)
        }
    } else if database_url_env.starts_with("sqlite:") {
        let file_part = &database_url_env["sqlite:".len()..];
        if file_part.starts_with('/') {
            PathBuf::from(file_part)
        } else {
            std::env::current_dir()
                .context("Failed to resolve current working directory")?
                .join(file_part)
        }
    } else {
        std::env::current_dir()
            .context("Failed to resolve current working directory")?
            .join(&database_url_env)
    };

    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create database directory {:?}", parent))?;
    }

    let database_display = database_path.display();
    let connect_options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .with_context(|| format!("Failed to connect to {}", database_display))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;

    seed_demo_data(&pool).await?;

    let log_path = Arc::new(PathBuf::from("server.log"));
    let state = AppState { 
        pool,
        log_path: log_path.clone(),
    };

    log_event(&log_path, &format!("Server starting on http://0.0.0.0:3000")).await;

    let app = Router::new()
        .route("/", get(index))
        .route("/style.css", get(style))
        .route("/app.js", get(app_js))
        .route("/api/table/metadata", get(table_metadata))
        .route("/api/table/:table", get(list_table).post(create_table_row))
        .route("/api/table/:table/:id", get(get_table_row).put(update_table_row).delete(delete_table_row))
        .route("/api/products", get(list_products))
        .route("/api/categories", get(list_categories))
        .route("/api/users/demo", post(create_demo_user))
        .route("/api/cart/:user_id", get(get_cart))
        .route("/api/cart/add", post(add_to_cart))
        .route("/api/orders/:user_id", get(list_orders))
        .route("/api/checkout", post(checkout))
        .layer(axum::middleware::from_fn_with_state(state.clone(), logging_middleware))
        .layer(axum::middleware::from_fn(cors_middleware))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on http://{}", addr);
    log_event(&log_path, &format!("Server listening on http://{}", addr)).await;

    let listener = TcpListener::bind(addr).await?;

    let log_path_clone = log_path.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("failed to listen for ctrl_c");
        log_event(&log_path_clone, "Shutdown signal received").await;
        std::process::exit(0);
    });

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn list_products(
    State(state): State<AppState>,
    Query(params): Query<ListProductsQuery>,
) -> ApiResult<Json<PaginatedProducts>> {
    let limit = params.limit.max(1).min(100);
    let offset = params.offset.max(0);

    let mut base_qb = QueryBuilder::new("SELECT p.* FROM products p LEFT JOIN product_categories pc ON p.id = pc.product_id WHERE p.active = 1");
    let mut count_qb = QueryBuilder::new("SELECT COUNT(DISTINCT p.id) FROM products p LEFT JOIN product_categories pc ON p.id = pc.product_id WHERE p.active = 1");

    if let Some(category_id) = &params.category_id {
        base_qb.push(" AND pc.category_id = ").push_bind(category_id);
        count_qb.push(" AND pc.category_id = ").push_bind(category_id);
    }

    if let Some(min_price) = params.min_price {
        base_qb.push(" AND p.price_cents >= ").push_bind(min_price);
        count_qb.push(" AND p.price_cents >= ").push_bind(min_price);
    }

    if let Some(max_price) = params.max_price {
        base_qb.push(" AND p.price_cents <= ").push_bind(max_price);
        count_qb.push(" AND p.price_cents <= ").push_bind(max_price);
    }

    if let Some(search) = &params.search {
        let search_pattern = format!("%{}%", search.replace('%', "\\%").replace('_', "\\_"));
        base_qb.push(" AND (p.title LIKE ").push_bind(search_pattern.clone()).push(" OR p.description LIKE ").push_bind(search_pattern.clone()).push(")");
        count_qb.push(" AND (p.title LIKE ").push_bind(search_pattern.clone()).push(" OR p.description LIKE ").push_bind(search_pattern.clone()).push(")");
    }

    base_qb.push(" GROUP BY p.id ORDER BY p.title LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);

    let total: i64 = count_qb
        .build_query_scalar::<i64>()
        .fetch_one(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    let mut items = base_qb
        .build_query_as::<schema::Product>()
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    // Apply best available discount per product (category-based or price-based)
    for product in items.iter_mut() {
        let percent = get_best_discount_percent(&state.pool, &product.id, product.price_cents)
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;
        if percent > 0 {
            product.price_cents = apply_percent(product.price_cents, percent);
        }
    }

    let has_more = offset + limit < total;

    Ok(Json(PaginatedProducts {
        items,
        total,
        limit,
        offset,
        has_more,
    }))
}

async fn list_categories(State(state): State<AppState>) -> ApiResult<Json<Vec<schema::Category>>> {
    let categories = sqlx::query_as::<_, schema::Category>("SELECT * FROM categories ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    Ok(Json(categories))
}

async fn get_best_discount_percent(pool: &SqlitePool, product_id: &Uuid, price_cents: i64) -> Result<i64, sqlx::Error> {
    let pid = product_id.to_string();
    let percent: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(percent) FROM discounts WHERE active = 1 AND (\
            (category_id IS NOT NULL AND category_id IN (SELECT category_id FROM product_categories WHERE product_id = ?))\
            OR\
            ((min_price_cents IS NULL OR min_price_cents <= ?) AND (max_price_cents IS NULL OR max_price_cents >= ?))\
        )"
    )
    .bind(pid)
    .bind(price_cents)
    .bind(price_cents)
    .fetch_one(pool)
    .await?;
    Ok(percent.unwrap_or(0))
}

fn apply_percent(price_cents: i64, percent: i64) -> i64 {
    price_cents - (price_cents * percent / 100)
}

async fn create_demo_user(State(state): State<AppState>) -> ApiResult<Json<PublicUser>> {
    let email = "demo@rustecom.local";
    if let Some(user) = sqlx::query_as::<_, schema::User>("SELECT * FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?
    {
        return Ok(Json(PublicUser {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            is_active: user.is_active,
        }));
    }

    let id = Uuid::new_v4();
    let password_hash = "demo-password-hash";
    sqlx::query("INSERT INTO users (id, email, password_hash, full_name, phone, is_active) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(id.to_string())
        .bind(email)
        .bind(password_hash)
        .bind("Demo User")
        .bind("+10000000000")
        .bind(1)
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(Json(PublicUser {
        id,
        email: email.to_string(),
        full_name: Some("Demo User".to_string()),
        is_active: true,
    }))
}

async fn get_cart(Path(user_id): Path<Uuid>, State(state): State<AppState>) -> ApiResult<Json<CartResponse>> {
    let cart_id = get_or_create_cart(&state.pool, user_id).await?;
    let items = sqlx::query_as::<_, CartItemResponse>(
        "SELECT ci.product_id, p.title, ci.quantity, p.price_cents as unit_price_cents, ci.quantity * p.price_cents as subtotal_cents FROM cart_items ci JOIN products p ON ci.product_id = p.id WHERE ci.cart_id = ? ORDER BY p.title",
    )
    .bind(cart_id.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::internal(err.to_string()))?;
    let mut items = items;

    // Apply discounts per item
    for item in items.iter_mut() {
        if let Ok(pid) = Uuid::parse_str(&item.product_id) {
            let percent = get_best_discount_percent(&state.pool, &pid, item.unit_price_cents)
                .await
                .map_err(|err| ApiError::internal(err.to_string()))?;
            if percent > 0 {
                item.unit_price_cents = apply_percent(item.unit_price_cents, percent);
                item.subtotal_cents = item.unit_price_cents * item.quantity;
            }
        }
    }

    let total_cents = items.iter().map(|item| item.subtotal_cents).sum();
    Ok(Json(CartResponse {
        cart_id,
        items,
        total_cents,
    }))
}

#[axum::debug_handler]
async fn add_to_cart(
    State(state): State<AppState>,
    Json(payload): Json<AddToCartRequest>,
) -> ApiResult<Json<CartResponse>> {
    if payload.quantity < 1 {
        return Err(ApiError::bad_request("Quantity must be at least 1"));
    }

    let cart_id = get_or_create_cart(&state.pool, payload.user_id).await?;

    let existing_quantity: Option<i64> = sqlx::query_scalar("SELECT quantity FROM cart_items WHERE cart_id = ? AND product_id = ?")
        .bind(cart_id.to_string())
        .bind(payload.product_id.to_string())
        .fetch_optional(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    if let Some(quantity) = existing_quantity {
        sqlx::query("UPDATE cart_items SET quantity = ? WHERE cart_id = ? AND product_id = ?")
            .bind(quantity + payload.quantity)
            .bind(cart_id.to_string())
            .bind(payload.product_id.to_string())
            .execute(&state.pool)
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;
    } else {
        sqlx::query("INSERT INTO cart_items (cart_id, product_id, quantity) VALUES (?, ?, ?)")
            .bind(cart_id.to_string())
            .bind(payload.product_id.to_string())
            .bind(payload.quantity)
            .execute(&state.pool)
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;
    }

    get_cart(Path(payload.user_id), State(state)).await
}

#[axum::debug_handler]
async fn checkout(
    State(state): State<AppState>,
    Json(payload): Json<CheckoutRequest>,
) -> ApiResult<Json<OrderSummary>> {
    let cart_id = get_or_create_cart(&state.pool, payload.user_id).await?;
    let items: Vec<CartItemResponse> = sqlx::query_as(
        "SELECT ci.product_id, p.title, ci.quantity, p.price_cents as unit_price_cents, ci.quantity * p.price_cents as subtotal_cents FROM cart_items ci JOIN products p ON ci.product_id = p.id WHERE ci.cart_id = ?",
    )
    .bind(cart_id.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::internal(err.to_string()))?;
    let mut items = items;

    if items.is_empty() {
        return Err(ApiError::bad_request("Cart is empty"));
    }
    // Apply discounts to items before computing totals and creating order
    for item in items.iter_mut() {
        if let Ok(pid) = Uuid::parse_str(&item.product_id) {
            let percent = get_best_discount_percent(&state.pool, &pid, item.unit_price_cents)
                .await
                .map_err(|err| ApiError::internal(err.to_string()))?;
            if percent > 0 {
                item.unit_price_cents = apply_percent(item.unit_price_cents, percent);
                item.subtotal_cents = item.unit_price_cents * item.quantity;
            }
        }
    }

    let total_cents: i64 = items.iter().map(|item| item.subtotal_cents).sum();
    let order_id = Uuid::new_v4();

    sqlx::query("INSERT INTO orders (id, user_id, shipping_address_id, billing_address_id, status, total_cents, currency) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(order_id.to_string())
        .bind(payload.user_id.to_string())
        .bind(payload.shipping_address_id.map(|id| id.to_string()))
        .bind(payload.billing_address_id.map(|id| id.to_string()))
        .bind("pending")
        .bind(total_cents)
        .bind("USD")
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    for item in items {
        sqlx::query("INSERT INTO order_items (id, order_id, product_id, quantity, unit_price_cents, subtotal_cents) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(order_id.to_string())
            .bind(item.product_id.to_string())
            .bind(item.quantity)
            .bind(item.unit_price_cents)
            .bind(item.subtotal_cents)
            .execute(&state.pool)
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;
    }

    sqlx::query("DELETE FROM cart_items WHERE cart_id = ?")
        .bind(cart_id.to_string())
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    let order = sqlx::query_as::<_, OrderSummary>("SELECT id, status, total_cents, currency, placed_at FROM orders WHERE id = ?")
        .bind(order_id.to_string())
        .fetch_one(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(Json(order))
}

async fn list_orders(Path(user_id): Path<Uuid>, State(state): State<AppState>) -> ApiResult<Json<Vec<OrderSummary>>> {
    let orders = sqlx::query_as::<_, OrderSummary>("SELECT id, status, total_cents, currency, placed_at FROM orders WHERE user_id = ? ORDER BY placed_at DESC")
        .bind(user_id.to_string())
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(Json(orders))
}

async fn get_or_create_cart(pool: &SqlitePool, user_id: Uuid) -> Result<Uuid, ApiError> {
    if sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE id = ?")
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?
        .is_none()
    {
        return Err(ApiError::not_found("User not found"));
    }

    if let Some(cart_id) = sqlx::query_scalar::<_, String>("SELECT id FROM carts WHERE user_id = ?")
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?
    {
        return Uuid::parse_str(&cart_id).map_err(|err| ApiError::internal(err.to_string()));
    }

    let cart_id = Uuid::new_v4();
    sqlx::query("INSERT INTO carts (id, user_id) VALUES (?, ?)")
        .bind(cart_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(cart_id)
}

struct TableMeta {
    columns: &'static [&'static str],
    pk: &'static [&'static str],
}

const TABLE_METADATA: &[(&str, TableMeta)] = &[
    (
        "users",
        TableMeta {
            columns: &["id", "email", "password_hash", "full_name", "phone", "is_active", "created_at", "updated_at"],
            pk: &["id"],
        },
    ),
    (
        "addresses",
        TableMeta {
            columns: &["id", "user_id", "label", "line1", "line2", "city", "region", "postal_code", "country", "is_default", "created_at", "updated_at"],
            pk: &["id"],
        },
    ),
    (
        "categories",
        TableMeta {
            columns: &["id", "slug", "name", "description", "created_at", "updated_at"],
            pk: &["id"],
        },
    ),
    (
        "products",
        TableMeta {
            columns: &["id", "sku", "title", "description", "price_cents", "currency", "active", "images", "created_at", "updated_at"],
            pk: &["id"],
        },
    ),
    (
        "discounts",
        TableMeta {
            columns: &["id", "name", "percent", "category_id", "min_price_cents", "max_price_cents", "active", "created_at", "updated_at"],
            pk: &["id"],
        },
    ),
    (
        "product_categories",
        TableMeta {
            columns: &["product_id", "category_id"],
            pk: &["product_id", "category_id"],
        },
    ),
    (
        "inventory",
        TableMeta {
            columns: &["product_id", "quantity", "reserved", "updated_at"],
            pk: &["product_id"],
        },
    ),
    (
        "carts",
        TableMeta {
            columns: &["id", "user_id", "created_at", "updated_at"],
            pk: &["id"],
        },
    ),
    (
        "cart_items",
        TableMeta {
            columns: &["cart_id", "product_id", "quantity", "added_at"],
            pk: &["cart_id", "product_id"],
        },
    ),
    (
        "orders",
        TableMeta {
            columns: &["id", "user_id", "shipping_address_id", "billing_address_id", "status", "total_cents", "currency", "placed_at", "fulfilled_at", "canceled_at", "created_at", "updated_at"],
            pk: &["id"],
        },
    ),
    (
        "order_items",
        TableMeta {
            columns: &["id", "order_id", "product_id", "quantity", "unit_price_cents", "subtotal_cents"],
            pk: &["id"],
        },
    ),
    (
        "payments",
        TableMeta {
            columns: &["id", "order_id", "method", "provider", "provider_transaction_id", "amount_cents", "status", "paid_at", "created_at", "updated_at"],
            pk: &["id"],
        },
    ),
];

fn get_table_meta(table: &str) -> Option<&'static TableMeta> {
    TABLE_METADATA.iter().find_map(|(name, meta)| (*name == table).then(|| meta))
}

fn parse_pk_values(id: &str, meta: &TableMeta) -> Option<Vec<String>> {
    let values: Vec<String> = id.split(',').map(|item| item.trim().to_string()).collect();
    if values.len() != meta.pk.len() {
        return None;
    }
    Some(values)
}

fn row_to_json(row: &sqlx::sqlite::SqliteRow, columns: &[&str]) -> anyhow::Result<Value> {
    let mut map = serde_json::Map::new();
    for &column in columns {
        let raw = row.try_get_raw(column)?;
        let value = if raw.is_null() {
            Value::Null
        } else if let Ok(n) = row.try_get::<i64, _>(column) {
            Value::Number(n.into())
        } else if let Ok(f) = row.try_get::<f64, _>(column) {
            Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0)))
        } else if let Ok(s) = row.try_get::<String, _>(column) {
            if let Ok(json) = serde_json::from_str::<Value>(&s) {
                json
            } else {
                Value::String(s)
            }
        } else {
            Value::Null
        };
        map.insert(column.to_string(), value);
    }
    Ok(Value::Object(map))
}

fn bind_value<'q>(mut query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>, value: &Value) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    if value.is_null() {
        query = query.bind(None::<String>);
    } else if let Some(n) = value.as_i64() {
        query = query.bind(n);
    } else if let Some(u) = value.as_u64() {
        query = query.bind(u as i64);
    } else if let Some(f) = value.as_f64() {
        query = query.bind(f);
    } else if let Some(b) = value.as_bool() {
        query = query.bind(if b { 1 } else { 0 });
    } else if let Some(s) = value.as_str() {
        query = query.bind(s.to_string());
    } else {
        query = query.bind(value.to_string());
    }
    query
}

async fn table_metadata(State(_state): State<AppState>) -> ApiResult<Json<Value>> {
    let tables: Vec<Value> = TABLE_METADATA
        .iter()
        .map(|(name, meta)| {
            serde_json::json!({
                "name": name,
                "columns": meta.columns,
                "primary_keys": meta.pk,
            })
        })
        .collect();
    Ok(Json(Value::Array(tables)))
}

async fn list_table(Path(table): Path<String>, State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let meta = get_table_meta(&table).ok_or_else(|| ApiError::bad_request("Unknown table"))?;
    let rows = sqlx::query(&format!("SELECT * FROM {}", table))
        .fetch_all(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    let mut list = Vec::new();
    for row in rows {
        list.push(row_to_json(&row, meta.columns).map_err(|err| ApiError::internal(err.to_string()))?);
    }
    Ok(Json(Value::Array(list)))
}

async fn get_table_row(Path((table, id)): Path<(String, String)>, State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let meta = get_table_meta(&table).ok_or_else(|| ApiError::bad_request("Unknown table"))?;
    let pk_values = parse_pk_values(&id, meta).ok_or_else(|| ApiError::bad_request("Invalid primary key format"))?;
    let where_clause = meta.pk.iter().map(|pk| format!("{} = ?", pk)).collect::<Vec<_>>().join(" AND ");
    let query_text = format!("SELECT * FROM {} WHERE {}", table, where_clause);
    let mut query = sqlx::query(&query_text);
    for value in pk_values.iter() {
        query = query.bind(value);
    }

    let row = query
        .fetch_optional(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    match row {
        Some(row) => Ok(Json(row_to_json(&row, meta.columns).map_err(|err| ApiError::internal(err.to_string()))?)),
        None => Err(ApiError::not_found("Row not found")),
    }
}

async fn create_table_row(Path(table): Path<String>, State(state): State<AppState>, Json(payload): Json<Value>) -> ApiResult<Json<Value>> {
    let meta = get_table_meta(&table).ok_or_else(|| ApiError::bad_request("Unknown table"))?;
    let mut payload_obj = payload.as_object().cloned().ok_or_else(|| ApiError::bad_request("Request body must be a JSON object"))?;

    if meta.pk == &["id"] && !payload_obj.contains_key("id") {
        payload_obj.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
    }

    let columns: Vec<&str> = payload_obj
        .keys()
        .filter(|key| meta.columns.contains(&key.as_str()))
        .map(|key| key.as_str())
        .collect();
    if columns.is_empty() {
        return Err(ApiError::bad_request("No valid columns provided"));
    }

    let placeholders = vec!["?"; columns.len()].join(", ");
    let sql = format!("INSERT INTO {} ({}) VALUES ({})", table, columns.join(", "), placeholders);
    let mut query = sqlx::query(&sql);
    for column in columns.iter() {
        query = bind_value(query, &payload_obj[*column]);
    }
    query
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    let pk_values: Vec<String> = meta
        .pk
        .iter()
        .map(|pk| {
            payload_obj
                .get(*pk)
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default()
        })
        .collect();

    get_table_row(Path((table, pk_values.join(","))), State(state)).await
}

async fn update_table_row(Path((table, id)): Path<(String, String)>, State(state): State<AppState>, Json(payload): Json<Value>) -> ApiResult<Json<Value>> {
    let meta = get_table_meta(&table).ok_or_else(|| ApiError::bad_request("Unknown table"))?;
    let payload_obj = payload.as_object().cloned().ok_or_else(|| ApiError::bad_request("Request body must be a JSON object"))?;
    let set_columns: Vec<&str> = payload_obj
        .keys()
        .filter(|key| meta.columns.contains(&key.as_str()) && !meta.pk.contains(&key.as_str()))
        .map(|key| key.as_str())
        .collect();

    if set_columns.is_empty() {
        return Err(ApiError::bad_request("No updatable columns provided"));
    }

    let set_clause = set_columns.iter().map(|col| format!("{} = ?", col)).collect::<Vec<_>>().join(", ");
    let where_clause = meta.pk.iter().map(|pk| format!("{} = ?", pk)).collect::<Vec<_>>().join(" AND ");
    let sql = format!("UPDATE {} SET {} WHERE {}", table, set_clause, where_clause);
    let mut query = sqlx::query(&sql);

    for col in set_columns.iter() {
        query = bind_value(query, &payload_obj[*col]);
    }

    let pk_values = parse_pk_values(&id, meta).ok_or_else(|| ApiError::bad_request("Invalid primary key format"))?;
    for value in pk_values.iter() {
        query = query.bind(value);
    }

    query
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    get_table_row(Path((table, id)), State(state)).await
}

async fn delete_table_row(Path((table, id)): Path<(String, String)>, State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let meta = get_table_meta(&table).ok_or_else(|| ApiError::bad_request("Unknown table"))?;
    let pk_values = parse_pk_values(&id, meta).ok_or_else(|| ApiError::bad_request("Invalid primary key format"))?;
    let where_clause = meta.pk.iter().map(|pk| format!("{} = ?", pk)).collect::<Vec<_>>().join(" AND ");
    let sql = format!("DELETE FROM {} WHERE {}", table, where_clause);
    let mut query = sqlx::query(&sql);
    for value in pk_values.iter() {
        query = query.bind(value);
    }

    let result = query
        .execute(&state.pool)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Row not found"));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn seed_demo_data(pool: &SqlitePool) -> anyhow::Result<()> {
    let product_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products")
        .fetch_one(pool)
        .await?;

    if product_count >= 500 {
        return Ok(());
    }

    let electronics_id = Uuid::new_v4();
    let kitchen_id = Uuid::new_v4();
    let fashion_id = Uuid::new_v4();
    let home_id = Uuid::new_v4();
    let sports_id = Uuid::new_v4();

    sqlx::query("INSERT OR IGNORE INTO categories (id, slug, name, description) VALUES (?, ?, ?, ?)")
        .bind(electronics_id.to_string())
        .bind("electronics")
        .bind("Electronics")
        .bind("Smart gadgets and devices")
        .execute(pool)
        .await?;

    sqlx::query("INSERT OR IGNORE INTO categories (id, slug, name, description) VALUES (?, ?, ?, ?)")
        .bind(kitchen_id.to_string())
        .bind("kitchen")
        .bind("Kitchen")
        .bind("Home and kitchen essentials")
        .execute(pool)
        .await?;

    sqlx::query("INSERT OR IGNORE INTO categories (id, slug, name, description) VALUES (?, ?, ?, ?)")
        .bind(fashion_id.to_string())
        .bind("fashion")
        .bind("Fashion")
        .bind("Clothing and accessories")
        .execute(pool)
        .await?;

    sqlx::query("INSERT OR IGNORE INTO categories (id, slug, name, description) VALUES (?, ?, ?, ?)")
        .bind(home_id.to_string())
        .bind("home")
        .bind("Home & Garden")
        .bind("Furniture and home decor")
        .execute(pool)
        .await?;

    sqlx::query("INSERT OR IGNORE INTO categories (id, slug, name, description) VALUES (?, ?, ?, ?)")
        .bind(sports_id.to_string())
        .bind("sports")
        .bind("Sports & Outdoors")
        .bind("Sporting goods and equipment")
        .execute(pool)
        .await?;

    let electronics_id: Uuid = Uuid::parse_str(
        &sqlx::query_scalar::<_, String>("SELECT id FROM categories WHERE slug = ?")
            .bind("electronics")
            .fetch_one(pool)
            .await?,
    )?;

    // Seed example discounts: 10% off electronics, 5% off items over $50 (5000 cents)
    sqlx::query("INSERT OR IGNORE INTO discounts (id, name, percent, category_id, min_price_cents, max_price_cents, active) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string())
        .bind("Electronics 10%")
        .bind(10)
        .bind(electronics_id.to_string())
        .bind(None::<i64>)
        .bind(None::<i64>)
        .bind(1)
        .execute(pool)
        .await?;

    sqlx::query("INSERT OR IGNORE INTO discounts (id, name, percent, category_id, min_price_cents, max_price_cents, active) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string())
        .bind("Over $50 5%")
        .bind(5)
        .bind(None::<String>)
        .bind(5000)
        .bind(None::<i64>)
        .bind(1)
        .execute(pool)
        .await?;

    let kitchen_id: Uuid = Uuid::parse_str(
        &sqlx::query_scalar::<_, String>("SELECT id FROM categories WHERE slug = ?")
            .bind("kitchen")
            .fetch_one(pool)
            .await?,
    )?;

    let fashion_id: Uuid = Uuid::parse_str(
        &sqlx::query_scalar::<_, String>("SELECT id FROM categories WHERE slug = ?")
            .bind("fashion")
            .fetch_one(pool)
            .await?,
    )?;

    let home_id: Uuid = Uuid::parse_str(
        &sqlx::query_scalar::<_, String>("SELECT id FROM categories WHERE slug = ?")
            .bind("home")
            .fetch_one(pool)
            .await?,
    )?;

    let sports_id: Uuid = Uuid::parse_str(
        &sqlx::query_scalar::<_, String>("SELECT id FROM categories WHERE slug = ?")
            .bind("sports")
            .fetch_one(pool)
            .await?,
    )?;

    for i in (product_count + 1)..=500 {
        let id = Uuid::new_v4();
        let sku = format!("FAKE-{i:03}");
        let title = format!("Fake Product {i}");
        let description = format!("A demo product with placeholder images for item {i}.");
        let price_cents = 1000 + ((i * 37) % 5000);
        let images = serde_json::json!([
            format!("https://picsum.photos/seed/{}/640/480", sku),
            format!("https://picsum.photos/seed/{}/640/480?1", sku),
            format!("https://picsum.photos/seed/{}/640/480?2", sku)
        ]);

        let images_json = serde_json::to_string(&images)?;
        let category_id = match i % 5 {
            0 => electronics_id,
            1 => kitchen_id,
            2 => fashion_id,
            3 => home_id,
            _ => sports_id,
        };

        sqlx::query("INSERT INTO products (id, sku, title, description, price_cents, currency, active, images) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(id.to_string())
            .bind(&sku)
            .bind(&title)
            .bind(&description)
            .bind(price_cents)
            .bind("USD")
            .bind(1)
            .bind(&images_json)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to insert fake product {i}"))?;

        sqlx::query("INSERT INTO product_categories (product_id, category_id) VALUES (?, ?)")
            .bind(id.to_string())
            .bind(category_id.to_string())
            .execute(pool)
            .await?;
    }

    Ok(())
}
