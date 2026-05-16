const userInfo = document.getElementById("user-info");
const createUserButton = document.getElementById("create-user");
const tableSelect = document.getElementById("table-select");
const operationSelect = document.getElementById("operation-select");
const idLabel = document.getElementById("id-label");
const rowIdInput = document.getElementById("row-id");
const bodyLabel = document.getElementById("body-label");
const requestBody = document.getElementById("request-body");
const fillSampleButton = document.getElementById("fill-sample");
const executeRequestButton = document.getElementById("execute-request");
const requestDisplay = document.getElementById("request-display");
const responseDisplay = document.getElementById("response-display");
const metadataContainer = document.getElementById("metadata");

// Product browser elements
const productSearchInput = document.getElementById("product-search");
const productCategorySelect = document.getElementById("product-category");
const productMinPriceInput = document.getElementById("product-min-price");
const productMaxPriceInput = document.getElementById("product-max-price");
const productLimitSelect = document.getElementById("product-limit");
const applyFiltersButton = document.getElementById("apply-filters");
const clearFiltersButton = document.getElementById("clear-filters");
const productsGrid = document.getElementById("products-grid");
const productsInfo = document.getElementById("products-info");
const productsPagination = document.getElementById("products-pagination");
const productApiEndpoint = document.getElementById("product-api-endpoint");

let currentUserId = localStorage.getItem("rustecom_user_id");
let tableMetadata = [];
let currentProductsPage = {
  offset: 0,
  limit: 20,
  filters: {},
};

function api(path, options = {}) {
  const headers = options.body ? { "Content-Type": "application/json" } : {};
  return fetch(path, { headers, ...options }).then(async (res) => {
    const text = await res.text();
    if (!res.ok) {
      let errorMessage = "API error";
      try {
        const body = JSON.parse(text);
        errorMessage = body.error || JSON.stringify(body);
      } catch (e) {
        errorMessage = text || errorMessage;
      }
      throw new Error(errorMessage);
    }
    if (!text) return null;
    try {
      return JSON.parse(text);
    } catch (e) {
      return text;
    }
  });
}

function setUserInfo(text) {
  userInfo.textContent = text;
}

function displayRequest(request) {
  requestDisplay.textContent = request;
}

function displayResponse(response) {
  responseDisplay.textContent = typeof response === "string" ? response : JSON.stringify(response, null, 2);
}

function updateFormState() {
  const operation = operationSelect.value;
  const needsId = operation === "get" || operation === "update" || operation === "delete";
  const needsBody = operation === "create" || operation === "update";

  idLabel.classList.toggle("hidden", !needsId);
  bodyLabel.classList.toggle("hidden", !needsBody);

  if (!needsId) {
    rowIdInput.value = "";
  }
  if (!needsBody) {
    requestBody.value = "";
  }
  updateRequestPreview();
}

function updateRequestPreview() {
  const table = tableSelect.value;
  const operation = operationSelect.value;
  const id = rowIdInput.value.trim();
  const body = requestBody.value.trim();
  let method = "GET";
  let path = `/api/table/${table}`;

  if (operation === "create") {
    method = "POST";
  } else if (operation === "get") {
    method = "GET";
    path += `/${encodeURIComponent(id)}`;
  } else if (operation === "update") {
    method = "PUT";
    path += `/${encodeURIComponent(id)}`;
  } else if (operation === "delete") {
    method = "DELETE";
    path += `/${encodeURIComponent(id)}`;
  }

  displayRequest(`${method} ${path}${body ? "\n\n" + body : ""}`);
}

function buildSampleBody() {
  const table = tableSelect.value;
  const meta = tableMetadata.find((item) => item.name === table);
  if (!meta) {
    requestBody.value = "";
    return;
  }

  const sample = {};
  meta.columns.forEach((column) => {
    if (meta.primary_keys.includes(column) && column === "id") {
      return;
    }
    if (column.endsWith("_id")) {
      sample[column] = "00000000-0000-0000-0000-000000000000";
    } else if (column.includes("price") || column.includes("amount") || column.includes("quantity")) {
      sample[column] = 100;
    } else if (column === "is_active" || column === "is_default") {
      sample[column] = true;
    } else if (column === "status") {
      sample[column] = "pending";
    } else {
      sample[column] = `${column}-value`;
    }
  });

  requestBody.value = JSON.stringify(sample, null, 2);
  updateRequestPreview();
}

async function loadMetadata() {
  try {
    const metadata = await api("/api/table/metadata");
    tableMetadata = metadata;
    tableSelect.innerHTML = metadata.map((table) => `<option value="${table.name}">${table.name}</option>`).join("");

    metadataContainer.innerHTML = metadata
      .map(
        (table) => `
          <div class="meta-card">
            <h3>${table.name}</h3>
            <p><strong>Columns:</strong> ${table.columns.join(", ")}</p>
            <p><strong>Primary keys:</strong> ${table.primary_keys.join(", ")}</p>
          </div>
        `,
      )
      .join("");

    updateRequestPreview();
  } catch (error) {
    displayResponse(error.message);
  }
}

async function createDemoUser() {
  try {
    const user = await api("/api/users/demo", { method: "POST" });
    currentUserId = user.id;
    localStorage.setItem("rustecom_user_id", user.id);
    setUserInfo(`Demo user: ${user.email}`);
    displayResponse(user);
  } catch (error) {
    displayResponse(error.message);
  }
}

async function executeRequest() {
  const table = tableSelect.value;
  const operation = operationSelect.value;
  const id = rowIdInput.value.trim();
  let path = `/api/table/${table}`;
  let method = "GET";
  const options = {};

  if (operation === "create") {
    method = "POST";
  } else if (operation === "get") {
    method = "GET";
    path += `/${encodeURIComponent(id)}`;
  } else if (operation === "update") {
    method = "PUT";
    path += `/${encodeURIComponent(id)}`;
  } else if (operation === "delete") {
    method = "DELETE";
    path += `/${encodeURIComponent(id)}`;
  }

  if (operation === "create" || operation === "update") {
    try {
      options.body = JSON.stringify(JSON.parse(requestBody.value));
    } catch (error) {
      displayResponse("Invalid JSON body: " + error.message);
      return;
    }
  }

  options.method = method;
  try {
    const result = await api(path, options);
    displayResponse(result);
  } catch (error) {
    displayResponse(error.message);
  }
}

async function loadCategories() {
  try {
    const categories = await api("/api/categories");
    productCategorySelect.innerHTML = '<option value="">All Categories</option>' + categories.map((cat) => `<option value="${cat.id}">${cat.name}</option>`).join("");
  } catch (error) {
    console.error("Failed to load categories:", error);
  }
}

async function loadProducts() {
  try {
    const params = new URLSearchParams();
    params.append("limit", currentProductsPage.limit);
    params.append("offset", currentProductsPage.offset);

    if (currentProductsPage.filters.search) {
      params.append("search", currentProductsPage.filters.search);
    }
    if (currentProductsPage.filters.category_id) {
      params.append("category_id", currentProductsPage.filters.category_id);
    }
    if (currentProductsPage.filters.min_price !== undefined && currentProductsPage.filters.min_price !== "") {
      params.append("min_price", Math.round(parseFloat(currentProductsPage.filters.min_price) * 100));
    }
    if (currentProductsPage.filters.max_price !== undefined && currentProductsPage.filters.max_price !== "") {
      params.append("max_price", Math.round(parseFloat(currentProductsPage.filters.max_price) * 100));
    }

    const result = await api(`/api/products?${params}`);
    updateApiEndpointDisplay(params);
    displayProducts(result);
  } catch (error) {
    productsGrid.innerHTML = `<div class="empty-state"><h3>Error loading products</h3><p>${error.message}</p></div>`;
  }
}

function updateApiEndpointDisplay(params) {
  const baseUrl = "/api/products";
  const queryString = params.toString();
  productApiEndpoint.textContent = `GET ${baseUrl}${queryString ? "?" + queryString : ""}`;
}

function displayProducts(result) {
  const { items, total, limit, offset, has_more } = result;

  if (items.length === 0) {
    productsGrid.innerHTML = '<div class="empty-state"><h3>No products found</h3><p>Try adjusting your filters</p></div>';
    productsInfo.textContent = "No results";
    productsPagination.innerHTML = "";
    return;
  }

  const startNum = offset + 1;
  const endNum = Math.min(offset + limit, total);
  productsInfo.textContent = `Showing ${startNum}–${endNum} of ${total} products`;

  productsGrid.innerHTML = items
    .map(
      (product) => `
    <div class="product-card">
      <img src="${product.images?.[0] || "data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 width=%22240%22 height=%22180%22%3E%3Crect fill=%22%23374151%22 width=%22240%22 height=%22180%22/%3E%3Ctext x=%2250%25%22 y=%2250%25%22 text-anchor=%22middle%22 dy=%22.3em%22 fill=%22%239ca3af%22 font-size=%2214%22%3ENo Image%3C/text%3E%3C/svg%3E"}" alt="${product.title}">
      <div class="product-info">
        <h3 class="product-title">${product.title}</h3>
        <div class="product-sku">SKU: ${product.sku}</div>
        <div class="product-price">$${(product.price_cents / 100).toFixed(2)}</div>
        ${product.description ? `<p class="product-description">${product.description}</p>` : ""}
      </div>
    </div>
  `,
    )
    .join("");

  // Pagination controls
  const pageNum = Math.floor(offset / limit) + 1;
  const totalPages = Math.ceil(total / limit);
  const paginationHtml = `
    <button ${offset === 0 ? "disabled" : ""} onclick="previousProductPage()">← Previous</button>
    <span class="pagination-info">Page ${pageNum} of ${totalPages}</span>
    <button ${!has_more ? "disabled" : ""} onclick="nextProductPage()">Next →</button>
  `;
  productsPagination.innerHTML = paginationHtml;
}

function nextProductPage() {
  currentProductsPage.offset += currentProductsPage.limit;
  loadProducts();
  window.scrollTo({ top: 0, behavior: "smooth" });
}

function previousProductPage() {
  currentProductsPage.offset = Math.max(0, currentProductsPage.offset - currentProductsPage.limit);
  loadProducts();
  window.scrollTo({ top: 0, behavior: "smooth" });
}

function applyProductFilters() {
  currentProductsPage.offset = 0;
  currentProductsPage.limit = parseInt(productLimitSelect.value);
  currentProductsPage.filters = {
    search: productSearchInput.value,
    category_id: productCategorySelect.value,
    min_price: productMinPriceInput.value,
    max_price: productMaxPriceInput.value,
  };
  loadProducts();
}

function previewApiEndpoint() {
  const params = new URLSearchParams();
  params.append("limit", productLimitSelect.value);
  params.append("offset", "0");

  if (productSearchInput.value) {
    params.append("search", productSearchInput.value);
  }
  if (productCategorySelect.value) {
    params.append("category_id", productCategorySelect.value);
  }
  if (productMinPriceInput.value) {
    params.append("min_price", Math.round(parseFloat(productMinPriceInput.value) * 100));
  }
  if (productMaxPriceInput.value) {
    params.append("max_price", Math.round(parseFloat(productMaxPriceInput.value) * 100));
  }

  const baseUrl = "/api/products";
  const queryString = params.toString();
  productApiEndpoint.textContent = `GET ${baseUrl}${queryString ? "?" + queryString : ""}`;
}

function clearProductFilters() {
  productSearchInput.value = "";
  productCategorySelect.value = "";
  productMinPriceInput.value = "";
  productMaxPriceInput.value = "";
  productLimitSelect.value = "20";
  currentProductsPage.offset = 0;
  currentProductsPage.limit = 20;
  currentProductsPage.filters = {};
  loadProducts();
}

createUserButton.addEventListener("click", createDemoUser);
operationSelect.addEventListener("change", updateFormState);
rowIdInput.addEventListener("input", updateRequestPreview);
requestBody.addEventListener("input", updateRequestPreview);
fillSampleButton.addEventListener("click", buildSampleBody);
executeRequestButton.addEventListener("click", executeRequest);

applyFiltersButton.addEventListener("click", applyProductFilters);
clearFiltersButton.addEventListener("click", clearProductFilters);
productLimitSelect.addEventListener("change", () => {
  currentProductsPage.limit = parseInt(productLimitSelect.value);
  currentProductsPage.offset = 0;
  loadProducts();
});

// Update API endpoint preview when filters change
productSearchInput.addEventListener("input", previewApiEndpoint);
productCategorySelect.addEventListener("change", previewApiEndpoint);
productMinPriceInput.addEventListener("input", previewApiEndpoint);
productMaxPriceInput.addEventListener("input", previewApiEndpoint);
productLimitSelect.addEventListener("change", previewApiEndpoint);

document.addEventListener("DOMContentLoaded", () => {
  if (currentUserId) {
    setUserInfo(`Stored demo user ID: ${currentUserId}`);
  }
  updateFormState();
  loadMetadata();
  loadCategories();
  previewApiEndpoint();
  loadProducts();
});
