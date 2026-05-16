# Running the Rustecom E-Commerce Backend

## Prerequisites

- Rust 1.70+ installed ([install from rustup.rs](https://rustup.rs/))
- Windows, macOS, or Linux

## Quick Start

### 1. Build the Project

```powershell
# From the RustEcom directory
cargo build --release
```

### 2. Run the Server

```powershell
cargo run --release
```

The server will start on `http://0.0.0.0:3000` and you'll see output like:

```
Listening on http://0.0.0.0:3000
```

### 3. Access the Web UI

Open your browser and navigate to:

```
http://localhost:3000
```

## Features

### Product Browser

- **Search** products by title or description
- **Filter** by category, price range
- **Pagination** with configurable page size (10, 20, 50, 100 items)
- Real-time API calls showing pagination parameters

### CRUD Test Panel

- Test all database tables directly from the UI
- Create, Read, Update, Delete operations
- See exact API requests and responses
- Sample data generator

### API Endpoints

#### Products (with pagination and filtering)

```
GET /api/products?limit=20&offset=0&category_id=XXX&min_price=1000&max_price=5000&search=keyword
```

Query Parameters:

- `limit`: Items per page (1-100, default 20)
- `offset`: Starting position (default 0)
- `category_id`: Filter by category UUID (optional)
- `min_price`: Minimum price in cents (optional)
- `max_price`: Maximum price in cents (optional)
- `search`: Search in title and description (optional)

#### Categories

```
GET /api/categories
```

#### Demo User (for testing)

```
POST /api/users/demo
```

#### Cart

```
GET /api/cart/:user_id
POST /api/cart/add
```

#### Orders

```
GET /api/orders/:user_id
POST /api/checkout
```

## Development

### Watch Mode

```powershell
cargo watch -x run
```

### Check Code Without Building

```powershell
cargo check
```

### View Logs

The server writes logs to `server.log` in the working directory.

## Database

The project uses SQLite with automatic migrations. The database is created at `ecommerce.db` (or as specified in `DATABASE_URL` environment variable).

### Sample Data

The server automatically seeds 500+ products across 5 categories on first run:

- Electronics
- Kitchen
- Fashion
- Home & Garden
- Sports & Outdoors

### Reset Database

To reset and reseed:

1. Delete `ecommerce.db`
2. Restart the server

## Troubleshooting

**Port 3000 already in use:**

```powershell
# Kill the process using port 3000
netstat -ano | findstr :3000
taskkill /PID <PID> /F
```

**Database locked error:**

- Ensure only one instance of the server is running
- Delete `ecommerce.db` and restart

**Compilation errors:**

```powershell
# Clean rebuild
cargo clean
cargo build --release
```
